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

//! `log_graph_args` accepts a list of commits in a repository, and computes
//! inclusions and exclusions to pass to `git log --graph` to show the
//! relationships between the given commits.

mod internal;

/// A list of inclusions and exclusions to be passed to git log.
#[derive(Clone)]
pub struct InclusionsExclusions {
    pub inclusions: Vec<git2::Oid>,
    pub exclusions: Vec<git2::Oid>,
}

pub fn log_graph_args(commits: &[git2::Commit]) -> InclusionsExclusions {
    let log_args = internal::gitloggraph(commits.iter().cloned().collect());
    InclusionsExclusions {
        inclusions: log_args.includes,
        exclusions: log_args.excludes,
    }
}
