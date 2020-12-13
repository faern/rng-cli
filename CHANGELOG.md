# Changelog
All changes to the software that can be noticed from the users' perspective should have an entry in
this file. Except very minor things that will not affect functionality, such as log message changes
etc.

### Format

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/).

Entries should have the imperative form, just like commit messages. Start each entry with words
like add, fix, increase, force etc.. Not added, fixed, increased, forced etc.

Line wrap the file at 100 chars.                                             That is over here -> |

### Categories each change fall into

* **Added**: for new features.
* **Changed**: for changes in existing functionality.
* **Deprecated**: for soon-to-be removed features.
* **Removed**: for now removed features.
* **Fixed**: for any bug fixes.
* **Security**: in case of vulnerabilities.


## [Unreleased]


## [0.2.0] - 2020-12-13
### Added
- Add multithreaded mode that is on by default, where the number of worker threads
  used is dynamically adjusted to saturate the stdout write speed.

### Removed
- Remove the possibility to explicitly specify the "default" algorithm. It can now only
  be selected by not specifying an algorithm.


## [0.1.0] - 2019-07-14
### Added
- Initial release. Supports singlethreaded mode, seeds, OS PRNG and a number of user space
  PRNGs.