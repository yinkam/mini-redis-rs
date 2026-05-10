# Known Issues

## EventLoop / buffer reads
- TCP handler doesn't handle partial reads properly
- Buffer dropped on failure instead of accumulated
- Works locally but will fail on real network conditions
- Fix before writing about event loop implementation