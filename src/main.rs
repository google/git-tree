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

// TODO: Pin Rust version, CI rustfmt, CI clippy, rustfmt config, clippy config,
// faster HashSet hasher.

use std::collections::HashSet;
use std::process::{Command, Stdio};
use std::str;

fn main() {
    // TODO: Spawn and stream in stdout? Skip UTF-8 checking?
    let output = Command::new("git")
        .args(&["branch", "--format=%(objectname)|%(upstream)"])
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
    let mut tracked = HashSet::new();
    for line in str::from_utf8(&output.stdout).expect("non-UTF-8 git output").lines() {
        let (id, upstream) = line.split_once('|').expect("Incorrect branch --format output");
        interesting.insert(id);
        if !upstream.is_empty() {
            tracked.insert(upstream);
        }
    }

    // TODO: Spawn and stream in stdout? Skip UTF-8 checking?
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .args(tracked)
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
    for line in str::from_utf8(&output.stdout).expect("non-UTF-8 git output").lines() {
        interesting.insert(line);
    }

    // TODO: Spawn and stream in stdout? Skip UTF-8 checking?
    let output = Command::new("git")
        .args(&["merge-base", "-a", "--octopus"])
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

    // TODO: Re-add command line config.
    Command::new("git")
        .args(&["log", "--graph", "--format=%C(auto)%h %d %<(50,trunc)%s"])
        .args(interesting)
        .arg("--not")
        .args(merge_bases.iter().map(|id| format!("{}^@", id)))
        .spawn()
        .expect("Failed to run git")
        .wait()
        .expect("git log failed");
}
