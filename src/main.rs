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

use core::str;
use std::collections::HashSet;
use std::process::{Command, Stdio};

#[allow(clippy::allow_attributes)] // TODO: Remove
#[allow(clippy::allow_attributes_without_reason)] // TODO: Remove
#[allow(clippy::expect_used)] // TODO: Re-evaluate
#[allow(clippy::print_stderr)] // TODO: Re-evaluate
#[allow(clippy::shadow_unrelated)] // TODO: Remove
#[allow(clippy::unwrap_used)] // TODO: Re-evaluate
fn main() {
    // TODO: Spawn and stream in stdout? Skip UTF-8 checking?
    let output = Command::new("git")
        .args(["branch", "--format=%(objectname)|%(upstream)"])
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
    let mut interesting = HashSet::new();
    for line in str::from_utf8(&output.stdout).expect("non-UTF-8 git output").lines() {
        let (id, upstream) = line.split_once('|').expect("Incorrect branch --format output");
        interesting.insert(id);
        if !upstream.is_empty() {
            interesting.insert(upstream);
        }
    }

    // TODO: Spawn and stream in stdout? Skip UTF-8 checking?
    let output = Command::new("git")
        .args(["merge-base", "-a", "--octopus", "HEAD"])
        .args(&interesting)
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
    let mut merge_bases = Vec::new();
    for line in str::from_utf8(&output.stdout).expect("non-UTF-8 git output").lines() {
        merge_bases.push(line);
    }

    // TODO: Stream.
    let output = Command::new("git")
        .args(["rev-list", "--parents", "--reverse", "--topo-order"])
        .args(&interesting)
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
    let mut visible: HashSet<&str> = merge_bases.iter().copied().collect();
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
