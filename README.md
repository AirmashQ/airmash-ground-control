## AIRMASH Ground Control

[![Build Status](https://travis-ci.org/AirmashPewPewPew/airmash-ground-control.svg?branch=master)](https://travis-ci.org/AirmashPewPewPew/airmash-ground-control)

Ground Control is an AIRMASH bot that sends wingmen to attack you. She's controlled through a chat interface. Ask her for help with `--gc-help`.

Let's get to it: request 3 wingmen from Ground Control with `--gc-wings 3`. You can request up to 5 wingmen to attack you. The wingmen are pretty dumb, always flying right to you, always shooting, and always predators. When you're done fighting your wingmen, say `--gc-call-off` to call them off your tail.

You'll note that you can only request wingmen to attack you; Ground Control doesn't want to bother players that don't want wingmen. Every player can request up to 5 wingmen, so hopefully the servers don't fall into chaos.

### Usage

We need a **nightly** Rust compiler to compile the binary (`cargo build [--release]`). We may also use the provided Dockerfile to get a Ground Control client up and running quickly.

```
$ airmash-ground-control ws://us.airmash.online/ffa1
```

will connect us to FFA1 in US. We may connect to as many servers as we'd like; just pass them in on the command line. Use `-h` / `--help` to ask for help and see all options.

Use a `RUST_LOG` environment variable to control logging outputs. The Docker image will, by default, show info messages and above.