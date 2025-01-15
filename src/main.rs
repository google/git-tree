// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A wrapper around `git log --graph` that heuristically determines what set of
//! commits should be displayed.

// The "interesting branches" are all local branches and all remote branches
// that are tracked by a local branch. The "interesting commits" are the commits
// pointed to by the interesting branches plus the HEAD commit. This tool
// displays the interesting commits, their collective merge bases, and any
// commits on the paths between the merge bases and the interesting commits.

use core::str;
use std::collections::HashSet;
use std::io::{BufRead as _, BufReader};
use std::process::{Command, Stdio};

/// Returns all interesting branches. Note that some commits may be in the list
/// multiple times under different names (different ref paths and/or a SHA
/// hash).
/// Precondition: `buffer` must be empty
/// Postcondition: `buffer` will be empty
fn interesting_branches(buffer: &mut Vec<u8>) -> HashSet<String> {
    let mut git = Command::new("git")
        .args(["branch", "--format=%(objectname)|%(upstream)"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to run git");
    let mut interesting = HashSet::new();
    let mut reader = BufReader::new(git.stdout.as_mut().unwrap());
    while let Some(len) =
        reader.read_until(b'\n', buffer).expect("git stdout read failed").checked_sub(1)
    {
        let (id, upstream) = str::from_utf8(buffer.get(..len).unwrap())
            .expect("non-utf-8 git output")
            .split_once('|')
            .expect("incorrect branch --format output");
        interesting.insert(id.to_owned());
        if !upstream.is_empty() {
            interesting.insert(upstream.to_owned());
        }
        buffer.clear();
    }
    drop(reader);
    let status = git.wait().expect("failed to wait for git");
    assert!(status.success(), "git returned unsuccessful status {status}");
    interesting
}

/// Returns all merge bases of the interesting commits.
/// Precondition: `buffer` must be empty
/// Postcondition: `buffer` will be empty
fn merge_bases(buffer: &mut Vec<u8>, interesting_branches: &HashSet<String>) -> Vec<String> {
    let mut git = Command::new("git")
        .args(["merge-base", "-a", "--octopus", "HEAD"])
        .args(interesting_branches)
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to run git");
    let mut merge_bases = Vec::with_capacity(1);
    let mut reader = BufReader::new(git.stdout.as_mut().unwrap());
    while let Some(len) =
        reader.read_until(b'\n', buffer).expect("git stdout read failed").checked_sub(1)
    {
        // Reserve enough space for the merge base plus a trailing ^@ (used in
        // the final `git log --graph` invocation).
        #[allow(
            clippy::arithmetic_side_effects,
            reason = "A line large enough to make this overflow does not fit in the address space"
        )]
        let mut merge_base = String::with_capacity(len + 2);
        merge_base
            .push_str(str::from_utf8(buffer.get(..len).unwrap()).expect("non-utf-8 git output"));
        merge_bases.push(merge_base);
        buffer.clear();
    }
    let status = git.wait().expect("failed to wait for git");
    assert!(status.success(), "git returned unsuccessful status {status}");
    merge_bases
}

#[allow(clippy::allow_attributes_without_reason)] // TODO: Remove
#[allow(clippy::print_stderr)] // TODO: Remove
#[allow(clippy::shadow_unrelated)] // TODO: Remove
fn main() {
    // Capacity estimate is a guess -- twice as large as a SHA-256 hash seems
    // reasonable (and is a power of two).
    let mut buffer = Vec::with_capacity(128);
    let interesting_branches = interesting_branches(&mut buffer);
    let merge_bases = merge_bases(&mut buffer, &interesting_branches);
    drop(buffer);

    // TODO: Stream.
    let output = Command::new("git")
        .args(["rev-list", "--parents", "--reverse", "--topo-order", "HEAD"])
        .args(&interesting_branches)
        .arg("--not")
        .args(&merge_bases)
        .stderr(Stdio::inherit())
        .output()
        .expect("Failed to run git");
    if !output.status.success() {
        eprintln!(
            "Git returned an unsuccessful status: {}. Git output:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout)
        );
        return;
    }
    let mut visible: HashSet<&str> = merge_bases.iter().map(String::as_str).collect();
    let mut includes: HashSet<&str> = HashSet::new();
    let mut stragglers: HashSet<&str> = HashSet::new();
    for line in str::from_utf8(&output.stdout).unwrap().lines() {
        let mut hashes = line.split(' ');
        let id = hashes.next().unwrap();
        includes.insert(id);
        for hash in hashes.clone() {
            includes.remove(hash);
        }
        if hashes.clone().any(|h| visible.contains(h)) {
            visible.insert(id);
            for hash in hashes.filter(|h| !visible.contains(h)) {
                stragglers.insert(hash);
            }
        }
    }

    // TODO: Re-add command line config.
    Command::new("git")
        .args(["log", "--graph", "--format=%C(auto)%h %d %<(50,trunc)%s"])
        .args(includes)
        .arg("--not")
        .args(merge_bases.iter().map(|id| format!("{id}^@")))
        .args(stragglers)
        .spawn()
        .expect("Failed to run git")
        .wait()
        .expect("git log failed");
}
