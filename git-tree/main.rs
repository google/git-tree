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

/// Computes and returns "interesting" commits -- the commits we would like to
/// show the history graph for.
fn get_interesting_commits(repository: &git2::Repository,
                           debug: bool) -> fnv::FnvHashSet<git2::Oid> {
    let mut interesting_commits = fnv::FnvHashSet::default();

    // HEAD is always interesting.
    interesting_commits.insert(repository.head()
                                         .expect("Unable to identify HEAD")
                                         .peel_to_commit()
                                         .expect("HEAD is not a commit")
                                         .id());

    // Utility function to add branch to interesting_commits.
    let mut add_branch = |branch: git2::Branch| interesting_commits.insert(
        branch.get()
              .peel_to_commit()
              .expect("Unable to resolve branch into commit")
              .id()
    );

    // Names of all local branches.
    let mut local_branch_names = fnv::FnvHashSet::default();

    // All remote branches.
    let mut remote_branches = Vec::new();

    // Iterate through all branches. At each step, repository.branches gives us
    // the branch object as well as the type of branch (local or remote).
    for (branch, branch_type) in repository.branches(None)
                                           .expect("Failed to identify branches")
                                           .map(|r| r.expect("Failed to open branch")) {
        // All local branches are assumed to be interesting.
        if branch_type == git2::BranchType::Local {
            local_branch_names.insert(branch.name_bytes().expect("No local branch name").to_vec());
            add_branch(branch);
            continue;
        }

        // The branch is remote; identify the remote name and the remote's
        // branch name
        let (_remote_name, base_name) = {
            let mut parts_iter = branch.name_bytes().expect("No remote branch name")
                                                    .splitn(2, |&b| b == b'/');
            (parts_iter.next().expect("Branch name missing remote"),
             parts_iter.next().expect("Branch name missing base"))
        };

        remote_branches.push((base_name.to_vec(), branch));
    }

    // Iterate through all remote branches. Remote branches that have the same
    // name as local branches are considered interesting.
    for (name, branch) in remote_branches {
        if local_branch_names.contains(&name) { add_branch(branch); }
    }

    if debug {
        println!("Interesting commits: {:?}", interesting_commits);
    }

    interesting_commits
}
fn main() {
    // TODO: Better document the command-line arguments.
    // TODO: See if replacing `clap` with another argument parser (perhaps a
    // custom parser?) improves the size of the executable.
    let cmdline_matches = clap::App::new("git-tree")
        .arg(clap::Arg::with_name("debug").long("debug").short("d"))
        .arg(clap::Arg::with_name("git_params").last(true).multiple(true))
        .get_matches();

    let repository = git2::Repository::open_from_env()
        .expect("Failed to open repository: is . a git repo?");
    let interesting_commits = get_interesting_commits(&repository,
                                                      cmdline_matches.is_present("debug"));

    // Transform the set of commits we want to show (interesting commits) into
    // the arguments we need to pass to git log to show them.
    if cmdline_matches.is_present("debug") {
        println!("Interesting commits: {:?}", interesting_commits);
    }
    let log_args = gitloggraph::log_graph_args(&interesting_commits.iter().map(|&id| {
        repository.find_commit(id).expect("Unable to find interesting commit")
    }).collect::<Vec<_>>());
    if cmdline_matches.is_present("debug") {
        println!("Includes: {:?}", log_args.inclusions);
        println!("Excludes: {:?}", log_args.exclusions);
    }

    // Finally: execute the `git log` command.
    let mut git_log_cmd = std::process::Command::new("git");
    git_log_cmd.args(&["log", "--graph"]);
    if let Some(values) = cmdline_matches.values_of_os("git_params") {
        git_log_cmd.args(values);
    }
    git_log_cmd.args(log_args.inclusions.iter().map(|c| c.to_string()));
    git_log_cmd.args(log_args.exclusions.iter().map(|c| format!("^{}", c)));
    git_log_cmd.spawn().expect("Failed to launch git log").wait().expect("git log failed");
}
