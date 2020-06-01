// Copyright 2020 Google LLC
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

/// git log doesn't have a nice way to show a set of commits and their
/// interconnections. It accepts an include/exclude list of commits, and
/// displays all commits that are reachable from commits in the include list and
/// are not reachable from commits in the exclude list.
///
/// gitloggraph() transforms from a list of interesting commits in a repository
/// to the git log arguments needed to display those commits (and their
/// interconnections).

#[path = "dag.rs"]
mod dag;

pub trait Commit: Sized {
    type Id: Copy + std::hash::Hash + Ord + std::fmt::Debug;
    type Timestamp: Copy + Ord;

    fn id(&self) -> Self::Id;
    fn parents(self) -> Vec<Self>;
    fn timestamp(&self) -> Self::Timestamp;
}

impl<'repo> Commit for git2::Commit<'repo> {
    type Id = git2::Oid;
    type Timestamp = git2::Time;

    fn id(&self) -> Self::Id { self.id() }
    fn parents(self) -> Vec<Self> { git2::Commit::parents(&self).collect() }
    fn timestamp(&self) -> Self::Timestamp { self.time() }
}

pub struct GitLogArgs<Id> {
    pub includes: Vec<Id>,
    pub excludes: Vec<Id>,
}

pub fn gitloggraph<C: Commit>(mut interesting_commits: Vec<C>) -> GitLogArgs<C::Id> {
    let num_interesting = interesting_commits.len();
    let nodes = dag::explore_dag(interesting_commits.drain(..).map(|c| {
        GitNode::new(c, true, num_interesting)
    }).collect());
    GitLogArgs::<C::Id> {
        includes: nodes.iter().filter(|n| n.interesting && n.reachable_from.len() == 1).map(|n| n.id.clone()).collect(),
        excludes: nodes.iter().filter(|n| n.exclude).map(|n| n.id.clone()).collect(),
    }
}

// -----------------------------------------------------------------------------
// gitloggraph() tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod gitloggraph_tests {
    use super::{Commit,gitloggraph};

    #[derive(Clone)]
    struct TestCommit<'c> {
        id: &'static str,
        parents: Vec<&'c TestCommit<'c>>,
        timestamp: u64,
    }

    impl<'c> Commit for TestCommit<'c> {
        type Id = &'static str;
        type Timestamp = u64;
        fn id(&self) -> Self::Id { self.id }
        fn parents(self) -> Vec<Self> { self.parents.iter().map(|&p| p.clone()).collect() }
        fn timestamp(&self) -> Self::Timestamp { self.timestamp }
    }

    #[test]
    fn empty() {
        let out = gitloggraph(Vec::<TestCommit>::new());
        assert!(out.includes.is_empty());
        assert!(out.excludes.is_empty());
    }

    #[test]
    fn one() {
        let a = TestCommit { id: "A", parents: Vec::new(), timestamp: 3 };
        let out = gitloggraph(vec![TestCommit{ id: "B", parents: vec![&a], timestamp: 7}]);
        assert_eq!(out.includes, vec!["B"]);
        assert_eq!(out.excludes, vec!["A"]);
    }
}

// -----------------------------------------------------------------------------
// Implementation details below.
// -----------------------------------------------------------------------------

struct GitNode<C: Commit> {
    commit: Option<C>,
    exclude: bool,
    fully_explored: bool,
    id: C::Id,
    interesting: bool,
    num_interesting: usize,
    priority: C::Timestamp,
    reachable_from: fnv::FnvHashSet<C::Id>,
    visible: bool,
}

impl<C: Commit> GitNode<C> {
    fn new(commit: C, interesting: bool, num_interesting: usize) -> Self {
        Self {
            exclude: false,
            fully_explored: false,
            id: commit.id(),
            interesting,
            num_interesting,
            priority: commit.timestamp(),
            reachable_from: if interesting { Some(commit.id()) } else { None }.iter().map(|&v| v).collect(),
            commit: Some(commit),
            visible: interesting,
        }
    }
}

impl<C: Commit> PartialEq for GitNode<C> {
    fn eq(&self, other: &Self) -> bool {
        self.exclude        == other.exclude        &&
        self.fully_explored == other.fully_explored &&
        self.reachable_from == other.reachable_from &&
        self.visible        == other.visible
    }
}

impl<C: Commit> dag::Node for GitNode<C> {
    type Id = C::Id;
    type Priority = C::Timestamp;

    fn id(&self) -> Self::Id { self.id }

    fn parents(&mut self) -> Vec<Self> {
        self.commit.take().expect("parents after update").parents().drain(..).map(|p| {
            Self::new(p, false, self.num_interesting)
        }).collect()
    }

    fn priority(&self) -> Self::Priority { self.priority }

    fn update(&self, children: &[&Self], parents: &[&Self], fully_explored: bool) -> Self {
        let reachable_from = children.iter().fold(self.reachable_from.clone(), |mut reachable, child| {
            for &id in &child.reachable_from { reachable.insert(id); }
            reachable
        });
        let visible = self.interesting || parents.iter().any(|p| p.visible) || (
            reachable_from.len() == self.num_interesting &&
            children.iter().all(|c| c.reachable_from.len() < self.num_interesting)
        );
        Self {
            commit: None,
            exclude: !visible && children.iter().any(|c| c.visible),
            fully_explored,
            id: self.id,
            interesting: self.interesting,
            num_interesting: self.num_interesting,
            priority: self.priority,
            reachable_from,
            visible,
        }
    }

    fn want_explore(&self) -> bool {
        !self.fully_explored && (self.visible || self.reachable_from.len() < self.num_interesting)
    }
}
