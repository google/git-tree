# gitree

`gitree` is a wrapper around `git log --graph` that heuristically determines
what set of commits should be displayed. It is designed for use with
branch-heavy workflows similar to those supported by the [Mercurial `evolve`
extension](https://www.mercurial-scm.org/wiki/EvolveExtension).

It accepts the following command-line flags:

* `--debug`/`-d`: Used for debugging `gitree`'s commit selection. Displays the
  commits that `gitree` is trying to show as well as the inclusion/exclusions
  arguments it is passing to `git log`.
* `--username`/`-u`: Specifies a username to look for to determine whether a
  remote repository is owned by the user invoking `gitree`. `gitree` shows all
  branches in this repository, rather than only branches corresponding to local
  branches.

Additionally, any remaining arguments after a `--` are passed through to `git
log`, allowing the user to set up their own formatting options.

For example, I have the following alias in my `.bashrc` to invoke `gitree`:

```
alias gitree='gitree -u jrvanwhy -- --format="%C(auto)%h %d %<(50,trunc)%s"'
```

This produces output similar to the following (albeit colorized by default,
which makes it much more readable than this example):
```
* 49aaffb  (origin/step7-last-futures, step7-last-futures) Remove the remaining uses of futures from..
* 4450f7b  (HEAD -> step6-buttons-2, origin/step6-buttons-2) Remove another use of button futures.
* f60a7dc  Add a dynamic call mechanism for client callbacks.
| * f2c8713  (lw2) More scratch work for virtualized time.
| | * 6e3e0c3  (origin/virtclk-scratch, virtclk-scratch) Work on virtualized lightweight clock.
| |/
|/|
* | 34282e5  (origin/step1-rng-size, step1-rng-size) Migrate the crypto library to the lightweight RN..
* | 5959c85  (origin/futures-size, futures-size) Add the size dump for the futures-based OpenSK!
* | 7f294f9  Start working on a lightweight RNG driver.
|/
* bdb3b8c  (origin/original-size, original-size) Store information about the size of the app with..
* e804c89  (origin/submods-to-dirs, submods-to-dirs) Replace the submodules with local directories. T..
* 57e79c1  (origin/master, origin/HEAD, master) Merge pull request #82 from jmichelp/master
```
