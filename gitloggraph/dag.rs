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

pub trait Node: PartialEq + Sized {
    type Id: Copy + std::hash::Hash + Ord + std::fmt::Debug;
    type Priority: Copy + Ord;

    fn id(&self) -> Self::Id;
    fn parents(&mut self) -> Vec<Self>;
    fn priority(&self) -> Self::Priority;
    fn update(&self, children: &[&Self], parents: &[&Self], fully_explored: bool) -> Self;
    fn want_explore(&self) -> bool;
}

struct Syncer<N: Node> {
    children: fnv::FnvHashMap<N::Id, Vec<N::Id>>,
    nodes: fnv::FnvHashMap<N::Id, N>,
    parents: fnv::FnvHashMap<N::Id, Vec<N::Id>>,
}

impl<N: Node> Syncer<N> {
    fn new() -> Self {
        Self {
            children: Default::default(),
            nodes: Default::default(),
            parents: Default::default(),
        }
    }

    fn add_node(&mut self, node: N, parents: Vec<N::Id>) {
        for &id in parents.iter() { self.children.entry(id).or_default().push(node.id()); }
        self.parents.insert(node.id(), parents);
        let children = self.children.entry(node.id()).or_default().clone();
        let parents: Vec<_> = self.parents[&node.id()].iter().filter(|p| self.nodes.contains_key(p)).map(|&v| v).collect();
        let mut to_update: fnv::FnvHashSet<N::Id> =
            [node.id()].iter()
            .chain(children.iter())
            .chain(parents.iter())
            .map(|&v| v).collect();
        self.nodes.insert(node.id(), node);
        while let Some(&id) = to_update.iter().next() {
            to_update.remove(&id);
            let mut children: Vec<_> = self.children.entry(id).or_default().clone()
                                       .iter().filter_map(|c| self.nodes.get(c)).collect();
            let mut parents: Vec<_> = self.parents[&id].iter().filter_map(|c| self.nodes.get(c)).collect();
            let new_state = self.nodes[&id].update(&children, &parents, parents.len() == self.parents[&id].len());
            if new_state != self.nodes[&id] {
                for id in children.drain(..).map(|c| c.id()).chain(
                          parents.drain(..).map(|p| p.id())) {
                    to_update.insert(id);
                }
            }
            self.nodes.insert(id, new_state);
        }
    }
}

pub fn explore_dag<N: Node>(mut interesting: Vec<N>) -> Vec<N> {
    let mut explore_cache: fnv::FnvHashMap<N::Id, N> = Default::default();
    let mut queue: std::collections::BTreeSet<(N::Priority, N::Id)> = Default::default();
    let mut syncer = Syncer::new();
    for mut node in interesting.drain(..) {
        let parents = node.parents().drain(..).map(|parent| {
            let id = parent.id();
            queue.insert((parent.priority(), id));
            explore_cache.entry(id).or_insert(parent);
            id
        }).collect();
        queue.remove(&(node.priority(), node.id()));
        syncer.add_node(node, parents);
    }
    while syncer.nodes.values().any(|n| n.want_explore()) {
        let &qe = queue.iter().next_back().expect("queue ran dry");
        queue.remove(&qe);
        let (_, id) = qe;
        if syncer.nodes.contains_key(&id) { continue; }
        let mut node = explore_cache.remove(&id).expect("cache entry missing");
        let parents = node.parents().drain(..).map(|parent| {
            let id = parent.id();
            if !syncer.nodes.contains_key(&id) { queue.insert((parent.priority(), id)); }
            explore_cache.entry(id).or_insert(parent);
            id
        }).collect();
        syncer.add_node(node, parents);
    }
    syncer.nodes.drain().map(|(_,v)| v).collect()
}
