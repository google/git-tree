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

use core::iter::repeat_n;
use core::str;
use std::collections::{HashMap, HashSet};
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
        interesting.insert(id.into());
        if !upstream.is_empty() {
            interesting.insert(upstream.into());
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
    drop(reader);
    let status = git.wait().expect("failed to wait for git");
    assert!(status.success(), "git returned unsuccessful status {status}");
    merge_bases
}

// We want to show the interesting commits, merge bases, and the commits on a
// path between the two. That is equivalent to showing all commits which
// satisfy:
// 1. The commit is reachable from an interesting commit, and
// 2. A merge base is reachable from the commit.
// This graph traversal computes the include and exclude arguments to pass to
// git log to show the above set of commits.
// We ask `git rev-list` to print all commits that are reachable from an
// interesting commit and not reachable from a merge base (note: this excludes
// the merge bases themselves). Every commit that git returns satisfies
// condition 1, but not all satisfy condition 2 (it may return commits that
// cannot reach a merge base).
// Since all such commits satisfy condition 1, we only really have to look at
// condition 2. If a commit can reach a merge base, then it should be shown, and
// we call it "visible". To easily compute which commits are visible, we ask git
// rev-list to print out the commits in reverse topological order, so that we
// visit all a commit's parents before we visit that commit. That way, when we
// visit a node, we know it is visible iff it has a visible parent.
// Once the graph traversal is complete:
// A) The includes list should consist of every childless visible commit.
// B) The excludes list should consist of every invisible commit that does not
//    have an invisible child.
// Fortunately, we can track whether a node has a (visible?) child as we
// traverse the graph. When we first add a commit, we mark it as having no
// (visible?) child, then we update that if we encounter its children. Note that
// we do not need to track invisible nodes that have invisible children -- they
// can be forgotten about entirely once detected.

#[derive(Clone, Copy, PartialEq)]
enum NodeState {
    // This node should not be visible in the final graph (it does not see a
    // merge base), and we have not yet explored any invisible child commits
    // of it. Note that InvisibleParent does not exist because if we find an
    // invisible child node of an InvisibleChild node, we remove the
    // InvisibleChild node entirely.
    InvisibleChild,

    // This node should be visible in the final graph (it does see a merge
    // base), and we've found a child node of it.
    VisibleParent,

    // This node should be visible in the final graph, and we have not yet
    // explored a child node of it.
    VisibleChild,
}
impl NodeState {
    /// Returns whether this is a visible node.
    fn is_visible(self) -> bool {
        self != Self::InvisibleChild
    }
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

    let mut git = Command::new("git")
        .args(["rev-list", "--parents", "--reverse", "--topo-order", "HEAD"])
        .args(&interesting_branches)
        .arg("--not")
        .args(&merge_bases)
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to run git");
    let mut nodes: Vec<_> = repeat_n(NodeState::VisibleChild, merge_bases.len()).collect();
    let mut free_slots = Vec::with_capacity(2);
    let mut node_lookup: HashMap<_, _> =
        merge_bases.iter().enumerate().map(|(i, id)| (id.clone().into(), i)).collect();
    // Indexes of the parents of this commit.
    let mut parents = Vec::with_capacity(2);
    let mut reader = BufReader::new(git.stdout.as_mut().unwrap());
    while let Some(len) =
        reader.read_until(b'\n', &mut buffer).expect("git stdout read failed").checked_sub(1)
    {
        // Iterate over the returned commit IDs. The first ID is the ID of this
        // commit, the rest are this commit's parents.
        let mut ids = buffer.get(..len).unwrap().split(|&b| b == b' ');
        // This commit's ID.
        let id = ids.next().expect("empty rev-list output line");
        parents.extend(ids.clone().map(|parent| node_lookup.get(parent).copied()));
        let visible = parents.iter().flatten().any(|&idx| nodes.get(idx).unwrap().is_visible());
        let new_state = if visible {
            for idx in parents.drain(..).flatten() {
                let parent = nodes.get_mut(idx).unwrap();
                if *parent == NodeState::VisibleChild {
                    *parent = NodeState::VisibleParent;
                }
            }
            NodeState::VisibleChild
        } else {
            for (parent, id) in parents.drain(..).zip(ids) {
                let Some(parent_idx) = parent else { continue };
                if nodes.get(parent_idx) != Some(&NodeState::InvisibleChild) {
                    continue;
                };
                node_lookup.remove(id);
                free_slots.push(parent_idx);
            }
            NodeState::InvisibleChild
        };
        match free_slots.pop() {
            None => {
                node_lookup.insert(id.to_vec(), nodes.len());
                nodes.push(new_state);
            }
            Some(new_idx) => {
                node_lookup.insert(id.to_vec(), new_idx);
                *nodes.get_mut(new_idx).unwrap() = new_state;
            }
        }
        buffer.clear();
    }
    drop(reader);
    drop(parents);
    drop(free_slots);
    drop(buffer);
    let mut includes = vec![];
    let mut excludes = vec![];
    for (id, idx) in node_lookup {
        match *nodes.get(idx).unwrap() {
            NodeState::InvisibleChild => {
                excludes.push(String::from_utf8(id).expect("non-utf-8 id"))
            }
            NodeState::VisibleChild => includes.push(String::from_utf8(id).expect("non-utf-8 id")),
            NodeState::VisibleParent => {}
        }
    }
    drop(nodes);
    let status = git.wait().expect("failed to wait for git");
    assert!(status.success(), "git returned unsuccessful status {status}");

    // TODO: Re-add command line config.
    Command::new("git")
        .args(["log", "--graph", "--format=%C(auto)%h %d %<(50,trunc)%s"])
        .args(includes)
        .arg("--not")
        .args(merge_bases.iter().map(|id| format!("{id}^@")))
        .args(excludes)
        .spawn()
        .expect("Failed to run git")
        .wait()
        .expect("git log failed");
}
