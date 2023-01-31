# gh-107528

This is a reproduction for https://github.com/rust-lang/rust/issues/107528#issuecomment-1410882750

Here's the valgrind output on my machine:

```
$ valgrind ./target/debug/gh-107528 
==140315== Memcheck, a memory error detector
==140315== Copyright (C) 2002-2017, and GNU GPL'd, by Julian Seward et al.
==140315== Using Valgrind-3.18.1 and LibVEX; rerun with -h for copyright info
==140315== Command: ./target/debug/gh-107528
==140315== 
WriteOwned::writev_all, calling writev with 2 items
self's type is gh_107528::TcpStream
==140315== Conditional jump or move depends on uninitialised value(s)
==140315==    at 0x127DD5: gh_107528::WriteOwned::writev::{{closure}} (main.rs:9)
==140315==    by 0x128293: gh_107528::WriteOwned::writev_all::{{closure}} (main.rs:20)
==140315==    by 0x128746: gh_107528::main::{{closure}} (main.rs:57)
==140315==    by 0x124EF3: tokio::runtime::park::CachedParkThread::block_on::{{closure}} (park.rs:283)
==140315==    by 0x124D41: tokio::runtime::park::CachedParkThread::block_on (coop.rs:102)
==140315==    by 0x1239F4: tokio::runtime::context::BlockingRegionGuard::block_on (context.rs:315)
==140315==    by 0x128C00: tokio::runtime::scheduler::multi_thread::MultiThread::block_on (mod.rs:66)
==140315==    by 0x125391: tokio::runtime::runtime::Runtime::block_on (runtime.rs:284)
==140315==    by 0x12998E: gh_107528::main (main.rs:55)
==140315==    by 0x12919A: core::ops::function::FnOnce::call_once (function.rs:250)
==140315==    by 0x12433D: std::sys_common::backtrace::__rust_begin_short_backtrace (backtrace.rs:121)
==140315==    by 0x1297B0: std::rt::lang_start::{{closure}} (rt.rs:166)
==140315== 
thread 'main' panicked at '`async fn` resumed after panicking', src/main.rs:9:82
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
==140315== Conditional jump or move depends on uninitialised value(s)
==140315==    at 0x12923B: core::ptr::drop_in_place<<gh_107528::TcpStream as gh_107528::WriteOwned>::writev::{{closure}}> (mod.rs:9)
==140315==    by 0x12825E: gh_107528::WriteOwned::writev_all::{{closure}} (main.rs:20)
==140315==    by 0x128746: gh_107528::main::{{closure}} (main.rs:57)
==140315==    by 0x124EF3: tokio::runtime::park::CachedParkThread::block_on::{{closure}} (park.rs:283)
==140315==    by 0x124D41: tokio::runtime::park::CachedParkThread::block_on (coop.rs:102)
==140315==    by 0x1239F4: tokio::runtime::context::BlockingRegionGuard::block_on (context.rs:315)
==140315==    by 0x128C00: tokio::runtime::scheduler::multi_thread::MultiThread::block_on (mod.rs:66)
==140315==    by 0x125391: tokio::runtime::runtime::Runtime::block_on (runtime.rs:284)
==140315==    by 0x12998E: gh_107528::main (main.rs:55)
==140315==    by 0x12919A: core::ops::function::FnOnce::call_once (function.rs:250)
==140315==    by 0x12433D: std::sys_common::backtrace::__rust_begin_short_backtrace (backtrace.rs:121)
==140315==    by 0x1297B0: std::rt::lang_start::{{closure}} (rt.rs:166)
==140315== 
==140315== 
==140315== HEAP SUMMARY:
==140315==     in use at exit: 15,634 bytes in 75 blocks
==140315==   total heap usage: 461 allocs, 386 frees, 111,668 bytes allocated
==140315== 
==140315== LEAK SUMMARY:
==140315==    definitely lost: 48 bytes in 1 blocks
==140315==    indirectly lost: 2 bytes in 2 blocks
==140315==      possibly lost: 0 bytes in 0 blocks
==140315==    still reachable: 15,584 bytes in 72 blocks
==140315==         suppressed: 0 bytes in 0 blocks
==140315== Rerun with --leak-check=full to see details of leaked memory
==140315== 
==140315== Use --track-origins=yes to see where uninitialised values come from
==140315== For lists of detected and suppressed errors, rerun with: -s
==140315== ERROR SUMMARY: 2 errors from 2 contexts (suppressed: 0 from 0)
```

Note that it shows up differently from the original report (panics on async fn
being resumed rather than segfault, I've also seen other failure modes, but
I believe they all have the same source).