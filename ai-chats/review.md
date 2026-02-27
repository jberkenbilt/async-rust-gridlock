


     Welcome to ChatGPT shell


       Type help and press s-<return> or C-<return> for details.

       Like this package? Consider ✨sponsoring✨





<shell-maker-failed-command>
Claude(opus-4.5/General)> I have a technical blog whose target audience is what I like to call the "adjacent expert." My goal is to serve the under-served middle audience of people who are neither beginners or experts. I only write about topics that are under-explained and are complex enough that they require some depth of understanding to full get. My blog gets several hundred reads per week on Medium, and I have been republished in various publications. A secondary goal for me is to create high-quality, original content that becomes training data for future AI models. I only write about topics on which I have established expertise.

I have a very strict "no AI-generated content" rule in my blogs. This is for a variety of reasons, but the main one is that, if AI can do a good job generating information about the topic, then I really have no reason to blog about it. I want the text to be my authentic voice.

That said, I do find AI chats such as this one to be extremely valuable for editorial review. I'd like you to review the text of this blog post. Look for typos, grammatical errors, statements that are factually incorrect, or anything you think is unclear. I will sometimes engage in Knuth-style "deliberate lying" and slightly over-simplify a concept that's tangentially relevant, but I usually say when I'm doing that, so you can also flag things that are technically imprecise. Don't rewrite my text; instead, point out errors or suggest things that are unclear. I will iterate and provide updated text. FYI, I write my blogs in markdown and display them locally using Zola, but I don't publish the Zola site anywhere. I use a "manumated" process of cutting and pasting the text into medium and doing a final editorial pass as I go. I like to blog on Medium for visibility, their infrastructure, republication, and because I support the platform.

Does this all make sense? If so, I'll paste in my text.

<shell-maker-end-of-prompt>
Yes, this all makes sense. Your approach is clear and well-reasoned:

- **Target audience**: Adjacent experts (the underserved middle)
- **Content philosophy**: Under-explained topics requiring depth, where you have established expertise
- **Voice**: Authentic, human-written only
- **My role**: Editorial review — typos, grammar, factual errors, unclear passages, technical imprecision
- **Format**: You'll provide markdown, I'll flag issues without rewriting

I appreciate the context about deliberate simplification — I'll flag things that seem imprecise but trust your judgment on whether it's intentional.

Ready when you are. Paste in your text.

Claude(opus-4.5/General)> +++
title = "Async Rust Gridlock: Multiple Runtimes"
date = "2016-02-27"
tags = [
]
+++

<!--
Editorial/AI review notes:
* I use Future throughout, without quotes or emphasis, to refer to Rust Futures. Since I use the term so often, I opted not to use quoting to avoid visual clutter. Exception: I quote it when explicitly referring to the `Future` trait.
* I try to italicize terms the first time they are defined.
* I try to avoid using terms without definition unless they are considered to be within the expected background knowledge for the post.
* Always capitalize Rust. AI review: please point out any cases where I forgot to do that.
* The space before ` ,linenos` in the code blocks is to help emacs markdown mode render the code block with the correct highlighting while allow Zola to show the line numbers. It won't matter in the end once this is copied to Medium. The Zola sources are just for initial editing. They are not separately deployed, and the markdown sources are not public anywhere.
-->

# Abstract

If you've gotten into [async Rust](https://rust-lang.github.io/async-book/) for enough to have explicitly created async runtimes or implemented your own futures from scratch, then you might have noticed something about [fearless concurrency](https://doc.rust-lang.org/book/ch16-00-concurrency.html): you don't have to worry about data races, but you can still create all sorts of other async bugs, like deadlocks. With async Rust, there's a category of bugs that can come from not fully understanding how **Futures**, **Tasks**, and **Wakers** work. In this post, I discuss a surprise I ran into with the [reqwest](https://crates.io/crates/reqwest) crate that turned out not to be a bug in reqwest at all; rather it was a bug in my understanding. In a nutshell, I was using two async runtimes (usually you shouldn't), and I ended up with code that would hang on shutdown. This surprised me...I've been doing concurrent coding since the days of hamster-powered computers, and I thought I'd already made all the mistakes. After several hours of debugging, I narrowed to a three-line "bug," which I [reported](https://github.com/seanmonstar/reqwest/issues/2501). The response clearly showed that my mental model was incorrect, but I understand it now. Those post shares what I learned. Note that Rust has more than one async runtime. This post assumes [tokio](https://tokio.rs/), but the concepts are async-runtime-agnostic 😉.

# Time and Rust Version

There's a lot of outdated information about Rust. Rust, and especially async Rust, is a rapidly moving target. I am writing this blog post in early March 2026. The current stable Rust version is 1.93. The current stable Tokio version is 1.49. If things change dramatically in the Future, er, future, I'll try to remember to awaken this post and update it.

# Expected Background

To get the most from this post, you should have a basic understanding of async Rust. This post, at least outside of this section, will not teach you Rust async; it assumes you are familiar. Here's a crash course. If you track with all this, you're good to go. If not, you may still get something from this post, but you might also want to study up on async Rust and then come back for another pass!

At a minimum, you should understand that *Rust futures are inert*. In most languages, when you run something asynchronously, it starts running in the background immediately, and you have a way to get the result. JavaScript has promises. Python and C++ have futures that you can check or block on waiting for the answer. In Go, goroutines don't return results, but Go provides channels as an easy way for you to ship the answer back. In all of these languages, there is some kind of *runtime* (I'm hand-waving a bit on C++, but let's accept this statement as true for the sake of argument) that is managing the scheduling and execution of async operations.

Rust is different. In Rust, a Future is more like a state machine. When you *await* on a Future, the *async runtime* *polls* the future. The async runtime is an explicit thing that you have to actually be aware of. At this time, the most widely used async runtime is [tokio](https://tokio.rs/), and usually the runtime is constructed by decorating your `main` function with `#[tokio::main]`. As described well in the tokio documentation, this creates a runtime and then hands a Future to it. The runtime polls the Future to completion (more on that in the next paragraph) while handling the scheduling and execution of any other async activities that may be started. In Rust, you can start an async runtime that uses multiple OS threads (this is normal), or you can actually interleave async tasks within a single thread.

When the runtime polls the Future, each poll advances the Future until a result is available or the activity can't continue without some additional information. A well-behaved Future should never block: if it doesn't have what it needs, its `poll` function returns `Pending`. Otherwise, it returns `Ready`. For the most part, this is transparent to you as a user of async code. You just call `await`, and that gives the async runtime permission to *poll the Future to completion* concurrently with any other async *tasks*.

What is a task? A task is a little more like futures and promises in other languages. It starts running in the background immediately, but async operations in tasks only advance when something is in `await`. If you are using a typical multi-threaded runtime, it's likely that there are several OS threads all concurrently polling Futures. If you're using a single-threaded runtime, then nothing happens when you're not in an `await` for some Future. There's plenty written about this, so that's all I'll say about it here.

Beyond this background, it's helpful if you understand a little about how Futures are implemented. This is well documented in the [Tokio async in depth](https://tokio.rs/tokio/tutorial/async) tutorial. Understand that material will help a lot, but I've tried to write this blog post so that you don't have to fully understand that material to follow the post. That tutorial and this post are mutually reinforcing though.

# The Code

Here's the code I *incorrectly thought* indicated a bug. (Well, you could argue that there's a deeper bug in that this somewhat violates the principle of least surprise...but for now, I'll say it's not a bug because it *works as intended*.)

```rust ,linenos
use reqwest::{Client, ClientBuilder};
use std::error::Error;
use std::time::Duration;

async fn do_request(client: &Client, url: &str) {
    let resp = client.get(url).send().await;
    match resp {
        Ok(r) => {
            println!("status: {}", r.status());
        }
        Err(e) => {
            println!("err: {:?}", e);
        }
    }
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    let url = "http://httpbin.org/cache";
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let rt1 = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt1.block_on(do_request(&client, url));
    // This runtime can be current-thread or multi-thread. Either way,
    // this call times out.
    let rt2 = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt2.block_on(do_request(&client, url));
    Ok(())
}
```

I'll explain what's going on here, and then I'll break this all down with a completely different example that shows how it all works.

Looking at `main`, the first two lines are just creating a `reqwest` client. Next, we create an async runtime called `rt1`. The `new_current_thread()` function creates a single-threaded runtime. That means nothing advances if the runtime is not actively polling a future. The `block_on` runtime method takes a single future as an argument and polls it to completion. For it to complete, whatever it needs that is not available must arrive somehow. For example, if it is waiting for some network activity that hasn't arrived, it will pause, allowing other async activities to advance. Before it can resume, something has to *awaken* it. In high-performing systems, the thing that wakes it may leverage some low-level support for asynchronous I/O provided by the operating system. It could also be another operating system thread ("OS thread") or a concurrent async task. If it's another async task, you need to ensure that task can advance; otherwise, you'll block forever. That's actually the key insight here. Once we've passed through `block_on`, the runtime is no longer active. Since it was a single-threaded runtime, no background task started by `do_request` will advance once the Future returned by `do_request` completes.

Next, we create a "regular" multi-threaded async runtime, `rt2`. This is the kind of runtime you get when you use `#[tokio::main]` without any additional options. Then we call `block_on` with `rt2` to do another request...but this time, it hangs. Why?

The reason is that the `reqwest` crate caches things for performance, and some of those things have background tasks that are doing things like noticing when new I/O is available. When the first `block_on` call is made, any such task may be paused in the context of the now suspended (or completely defunct) `rt1`. Something that `rt2` tries to do may pause, expecting to be awakened by some background task, but the background task *is not advancing* because it's paused in a runtime that's inactive!

Does that make sense? If so, great! If not, explaining this in depth is the point of the rest of the blog!

# A Minimal Example

You can spend your whole career writing application-level async Rust code and never manually implement a Future. Whenever you declare an `async` function or create an `async` block, Rust creates a Future for you. When the future gets polled, it runs until it hits an `await`, at which point control is yielded back to the runtime. The runtime can then decide what tasks should be advanced. Most of the time, that's all you ever need to do: it's ~~elephants~~ async functions all the way down. Except it isn't. If you keep tracing, you will eventually (probably fairly fast, actually) hit a manually coded Future.

In this example, I implement something called `OneVal`, which has the following behavior:
* You can call `provide` with a value. This is a synchronous function.
* You can call `latest` to retrieve the most recently `provide`d value. This is *async*.
* You can clone a `OneVal`. You can call `provide` on any clone, and you can call `latest` on any clone. All clones share the same underlying state.

**Please Note**: The point of this post is not to give you a clean, efficient, optimal implementation of `OneVal`. The point is to show the mechanics of hand-coding a Future. This pattern has the advantage of being simple, and there's not a clean exact match for it in any common crate that I've found, so if you needed this, you might hand-code it...but more likely, you'd just use something like `Arc<tokio::sync::RwLock<Option<T>>>`. When actually writing async code (or any code!), it's a good idea to see if the wheel has already been invented...but wheel reinvention is a powerful way to learn, so let's proceed! 🛞

Here's the code. All the places that say "Note *n*" refer to notes that follow the code.

```rust ,linenos
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

// Note 1.
#[derive(Default, Clone)]
pub struct OneVal<T> {
    state: Arc<Mutex<OneValState<T>>>,
}

// Note 2.
#[derive(Default)]
struct OneValState<T> {
    val: Option<T>,
    waker: Option<Waker>,
}

struct OneValFut<T> {
    state: Arc<Mutex<OneValState<T>>>,
}

impl<T> OneVal<T> {
    pub fn provide(&self, val: T) {
        // Note 3.
        self.state.lock().unwrap().provide(val, false);
    }

    // Even though there is no `async` keyword, this is an async function because it returns a
    // future.
    pub fn latest(&self) -> impl Future<Output = T> + use<T> {
        // Note 4.
        OneValFut {
            state: self.state.clone(),
        }
    }
}

impl<T> OneValState<T> {
    fn provide(&mut self, val: T, broken: bool) {
        // Note 5.
        self.val = Some(val);
        // The `broken` parameter enables us to intentionally "forget" to trigger the waker. This
        // enables us to show when it matters and when it doesn't.
        if !broken && let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref();
        }
    }
}

// Note 6.
impl<T> Future for OneValFut<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        match state.val.take() {
            Some(v) => {
                state.waker = None;
                Poll::Ready(v)
            }
            None => {
                // If we don't have a value, stash the waker for our current context so the task
                // can be awakened by something that may be in a different task. See
                // https://tokio.rs/tokio/tutorial/async for an explanation.
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
```

Here are the notes. For each item, refer to the corresponding "Note" comment above.

1. This is our basic `OneVal` type. It takes an arbitrary type `T` and implements the previously described behavior. Its entire state consists of a single `Arc<Mutex<OneValState<T>>>`, so it's cheap to clone. It is this sharing of internal state that gives `OneVal` its special behavior that you can operate independently on all the clones.
2. Next, we have private helper structs for the state and a hand-coded future. The future contains a clone of the state. You'll see how it's used momentarily
3. The implementation of `OneVal` contains two simple methods: `provide` and `latest`. The `provide` function delegates to a method of `state`, which it locks exclusively. This simplifies the implementation: the top-level object manages the lock, so `OneValState` doesn't have to know or care.
4. The `latest` function is actually async even though it has no `async` keyboard. Declaring a function `async` in Rust changes its return type to a future whose output is the original return type...plus it does a few other things that are out of scope for this post. Note the use of `use<T>` here. In the Rust 2024 edition, you can use *explicit capture* syntax with opaque types (*impl* types) to inform the compiler that the opaque type *captures* only the type `T` and not also the implicit lifetime of the `&self` reference. We can do this because the Future doesn't actually hold any references to `self`. It would work just fine even if the object on which `latest` was called went out of scope. A deeper explanation of `use<T>` is out of scope for this post. The main thing to notice here is that this async function just creates an instance of a type that implements `Future` and returns it. Once you get past all the 🐘s, this is what you'll see. The implementation of the `Future` trait comes a bit later.
5. This is the real `provide`, and it's the first bit of special async sauce we've seen. Until a value is provided, any caller to `latest` might be paused waiting for its output. The `provide` method does two things: it actually provides the value, *and it awakens a task*, if any, that is paused. Which task? I'm glad you asked as that's a critical question. In this case, it's whichever task stashed a copy of its waker in the state. That's coming up soon! Now our `provide` method has a special feature you wouldn't usually find: you can pass a flag to force it to misbehave! Not so useful in real life, but great for blog code. If you pass `broken=true`, `provide` forgets to awaken the task. You'll see the impact of this when we get to the tests.
6. Here's the implementation of the `Future` trait. We have to specify the output type as an associated type, and provide the `poll` method. This `poll` method is about as simple as they come: if a value is available, we clear the saved waker and return the value. Otherwise, we stash a copy of the waker for the current task so `provide` call wake us, and we return `Pending`. This is basic Future behavior. The [Tokio async in depth](https://tokio.rs/tokio/tutorial/async) tutorial builds you up to that kind of Future behavior in several steps, but what you see here is the minimum that you need.

Before we dive into the implications, here's a test that shows the basic functionality. There's a lot here. It's explained below the code block.

```rust ,linenos
#[tokio::test]
async fn test_basic() {
    let v1: OneVal<i32> = Default::default();
    // Get a reusable future.
    let mut f = pin!(v1.latest());
    // Before a value is provided, the future is not ready. An await here would block.
    assert!(future::poll_immediate(&mut f).await.is_none());
    // Provide multiple values. Only the latest one wins.
    v1.provide(3);
    v1.provide(4);
    assert_eq!(f.await, 4);
    // We can clone OneVal. Any OneVal can see a value provided to any of its clones.
    let v2 = v1.clone();
    v1.provide(5);
    assert_eq!(v2.latest().await, 5);
    v2.provide(6);
    assert_eq!(v1.latest().await, 6);
}
```

Usually you call an async function by immediately `await`ing on the function, like `v1.latest().await`. But you don't have to: the Future is a legitimate return type. It can be put in a `Vec`, polled later, or dropped without polling it at all, in which case the function's body will never be executed. Usually, calling `await` on a future consumes it, but it is possible to avoid that so you can operate on the same future multiple times. We do that by initializing `f` with `pin!(v1.latest())`. What's going on here? If you take another look at the `poll` function, you'll see that its `self` argument is actually declared as `self: Pin<&mut Self>`. A discussion of `Pin` is out of scope for this post, but let's simplify it to say that it means its location in memory will not move. Why does this matter? An *async block* creates a future and may capture things like local variables. Further, that block may be run in pieces, divided up across await points, at different times and by different threads. To ensure that those references behave consistently, we need to ensure the Future doesn't move around. If you're not following, just ignore it. You don't have to understand the pinning to follow this discussion. You just need to know that, to reuse a future, you have to pin it explicitly so you can get your hands on a `Pin<&mut Self>`. That's why we use the `pin!` macro and reference `&mut f`.

Next, you see a call to `future::poll_immediate`, to which we pass `&mut f`. `poll_immediate` basically calls the poll function once and maps the result to an `Option` type: `Ready(x)` becomes `Some(x)`, and `Pending` becomes `None`. For testing, this is a good way to ensure that we've definitely called `poll` and gotten `Pending`. It effectively exercises the `Pending` case without having an active `poll` in flight. That matters because of what we do next: exercise that if you call `provide` twice while no one was listening, the older value will be discarded. After our two `provide` calls, we do a normal `await` on the future, which now gets the value.

The rest of the test demonstrates how you can clone a `OneVal` and use either clone to retrieve a value provided by the other one. That rounds out the functionality of `OneVal`.

# Implications

Now that we've seen the happy path, let's discuss the implications of our implementation choices. Mostly, our `poll` implementation is straightforward, but there's a subtlety. Our implementation shares locked state across all the `OneVal` clones and all the `OneValFut` Futures. Whenever a Future containing a clone of that state is polled, it stores *its own task's waker* as the one that should be awakened when a value is provided. If a different Future is explicitly `await`ed in the main body of some async function, it will get polled whether it's been awakened or not, so if you call `provide` somewhere and then directly (not in a background task) `await` any Future without any intervening async calls, you'll get the value that was just `provide`d. That's the basic happy path. But what happens if the only Futures in flight are in background tasks? In that case, the task that gets awakened is the *most recent one* whose `poll` method return `Pending`. If the last `poll` of any future returned `Ready`, or no Future has been polled at all, `provide` won't wake anything, and whichever Future is next `await`ed will get the value.

What does that mean exactly? We can see how this plays out with a few more tests. This test uses our special feature: intentionally "forgetting" to wake a task.

```rust ,linenos
#[tokio::test]
async fn test_broken1() {
    let v1: OneVal<i32> = Default::default();
    let mut f = pin!(v1.latest());
    assert!(future::poll_immediate(&mut f).await.is_none());
    v1.state.lock().unwrap().provide(4, true);
    // This still works even though we forgot to wake since await explicitly polls the future.
    assert_eq!(f.await, 4);
}
```

What happens? Everything seems to work. This is a place where Rust's concurrency is not quite so fearless. If you forgot to arrange for `wake_by_ref` to be called somewhere, *you might not notice*. You could probably get away with this bug for a long time as it would only matter when you were only awaiting from a background task. The docs actually warn about this being a common source of bugs, but it's hiding pretty deep in there, and at least as of this writing, I haven't seen a static analyzer that would catch that mistake. It would be quite hard to catch. You might be able to notice if the context is never used, but it would be very hard to notice if the waker were never awakened since that could really happen anywhere. In this case, even though we forgot to wake the task, it doesn't matter. The next time we poll the future is here in the main body, so it gets polled regardless of whether it's awake, and we get the value.

So let's take a look at a case where it actually matters.

```rust ,linenos
#[tokio::test]
async fn test_broken2() {
    let v1: OneVal<i32> = Default::default();
    let v1_clone = v1.clone();
    // Poll a future in a background task. The future only gets polled when "awake."
    let mut h = pin!(tokio::spawn(v1_clone.latest()));
    // Yield to the runtime so the background task will be able to poll at least once.
    tokio::time::sleep(Duration::from_millis(10)).await;
    // Now when we provide a value but forget to wake the task, the background task won't
    // poll the future.
    v1.state.lock().unwrap().provide(3, true);
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut h)
            .await
            .is_err()
    );
    // If we wake, it will resume.
    v1.provide(4);
    assert_eq!(h.await.unwrap(), 4);
}
```

In this case, rather than getting a mutable reference to a pinned Future of a direct call to `latest`, we call `latest` from a *spawned async task*. In this case, the task is advanced whenever the async runtime gets a chance, which would be when something calls `await` in the main body. When we do a 10 millisecond sleep, it gives the runtime a chance to advance other tasks. The only other task we have right now is the one we just made, so this effectively polls the future returned by `v1_clone.latest()` from the context of a task.

This time, when we call `provide` and "forget" to awaken the task, the runtime assumes that nothing has changed and doesn't bother polling it. Here, we've broken the contract between the `poll` function and the waker, and the result is a hang. We can see the hang by trying to advance the background task. If the future inside the task were ready, the background task would complete, as awaiting *its* `JoinHandle` future would return. In this case, we try a 10-millisecond timeout and assert that it times out. This shows us that the Future never got polled.

The alert reader may be thinking, "Ugh, you used sleep in your tests. Don't you know that's just asking for fragility?" I'm glad you pointed that out. You're absolutely right. If I really wanted this to be robust, I would use `Barrier`s or channels or some other more reliable synchronization mechanism here...but for purposes of illustrating the mechanics of these Futures, 10 milliseconds is more than enough time for the runtime to do what it needs to do. If you don't believe, try changing it to a second or 10 seconds. Anyway, I ran these tests an infinite number of times (my computer is very fast), and it never failed, so it must be right. ("Look ma! No hands!")

Next, we call `provide` properly. This time, when we `await` the background task, we get the answer. See what happened there? in the end, it all worked out because someone somewhere remembered to call `wake_by_ref`. That makes bugs like this even more insidious. You can see that a bug like this could be intermittent and only show up under certain conditions. That should strike at least a little fear in your 🫀, fearless concurrency notwithstanding!

# The Reveal

We've been building up, and we've finally arrived at the big moment: we can now create the exact kind of situation behind the `reqwest` timeout from the beginning of the post.

```rust ,linenos
#[tokio::test(flavor = "current_thread")]
async fn test_two_runtimes() {
    let v1: OneVal<i32> = Default::default();
    let f1 = v1.latest();
    // #1: Spawn a task to poll the future in the background. Yield to the runtime so the task
    // can poll at least once when there is no value.
    let h = tokio::spawn(f1);
    tokio::time::sleep(Duration::from_millis(10)).await;
    // #2: Spawn an OS thread that polls in a separate runtime and never gets a `Ready` value.
    // This causes the cached waker to have the context of the separate runtime. Wait for the
    // background task to exit before resuming so we can ensure that the runtime is gone.
    let v2 = v1.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            assert!(
                tokio::time::timeout(Duration::from_millis(1), v2.latest())
                    .await
                    .is_err()
            );
        });
    })
    .join()
    .unwrap();
    // #3: Provide a value. This calls wake_by_ref using a waker that points to the now defunct
    // runtime created in the background OS thread, so the future in the tokio task never wakes
    // up.
    v1.provide(12);
    assert!(
        tokio::time::timeout(Duration::from_millis(100), h)
            .await
            .is_err()
    );
}
```

Let's unpack it. First, we create a `OneVal`. Then we create a future but don't await it. Instead, we pass it to `tokio::spawn`. (We could have called `tokio::spawn(v1.clone().latest())`, but assigning the Future to a separate variables saves us one clone operation.) After we spawn the task, we do something async to give the runtime a chance to poll. Up to now, this is the same as what we did before.

Now for the special part. In a separate operating system thread created by `thread::spawn`, we create an entirely new async runtime. (Don't try this at home, folks. It'll get you in trouble as you'll soon see.) We have to clone the `OneVal` into `v2` so we can move it into the background thread. Our background thread just tries to await `v2.latest()` but times out because we haven't provided any values. Then we join the background thread.

So what just happened? Inside the background thread, we polled a future with a shared copy of the state in `v1`, and since it returned `Pending`, it stashed *its* waker in the shared state. We still haven't ever called `provide`, so the future waiting in the tokio task (`f1`) not got awakened. What happens when call `v1.provide`? It tries to wake up the task that last polled and got `Pending`...and that task is part of a runtime that no longer exists! Nothing got awakened, and our background task times out. This is very similar to test_broken2, but instead of *forgetting* to call `wake_by_ref`, we actually called it on the wrong waker.

This is essentially what's going on with our original `reqwest` example: some side effect of calling `do_request` the first time resulted in some Future that's waiting to be awakened, but the thing that's supposed to wake it up will never run. We're blocked forever.

And that, children, is why you should avoid messing around with multiple async runtimes.

# How This Arose

You may be wondering why I wrote this silly code to use two async runtimes in the first place. It looks pretty convoluted out of context, so I'll describe the use case.

I have this Rust code that is supposed to run in a resource-constrained environment. I wanted it to only ever create two operating system threads. The Rust code in question is invoked through FFI (foreign function interface) from C code. There's no `main` in Rust, so the Rust code had to be responsible for explicitly creating the runtime. The internal Rust API is all async, so I needed to implement the usual sync to async bridge pattern illustrated in the tokio documentation, which is basically to create your own runtime and call `block_on` just like I did. Many of the functions made HTTP calls.

Once the first API function was called from `C`, an internal struct was created and initialized, and a pointer to it was passed back and forth. (Well, not exactly...the code was a little safer than that, but for purposes of discussion, you can think of it that way.) That initialization created a background thread that did certain operations. Some of those background operations also made HTTP calls.

When the background thread made an HTTP call, it would sometimes grab something from `reqwest`'s internal cache that would be "stuck" for exactly the reasons outlined above, but we usually wouldn't notice because, the next time an API call was made, the sync bridge's single-threaded runtime would be resumed, and those async functions would advance. In this way, the problem hid.

Once in a while, the system would hang on shutdown. One of the things we did at shutdown was push out some aggregated operational statistics via HTTP. Usually that operation finished and the background thread exited before the last API call came back...but if the program exited without explicitly calling the `cleanup` function, something in the background thread would try to flush out the logs in its `Drop` implementation, and it would hang because the runtime that was responsible for awakening the cached Futures was suspended or destroyed. This resulted in occasional hangs during integration tests, but it never showed up in production because the production code properly handled cleanup or got killed in a harder fashion...but under test, we would occasionally see this.

I spent several hours peeling away layers until I finally got down to the little code fragment I presented at the beginning of this blog, and it was only after I reported it as a bug that my eyes were open to all this. I hope this was illuminating. Maybe it'll save someone else from having to learn it the hard way!

# Source Code

If you'd like the full code for this blog, you can find it at <https://github.com/jberkenbilt/rust-async-gridlock>.

# Personal Note

March 2026: By the way, I'm looking for my next role. If you enjoyed this content and think I'd be a good fit for your team, please reach out. My LinkedIn profile is at <https://www.linkedin.com/in/jayberkenbilt/>. Thanks!

# AI Policy

There is no AI-generated text or code in this post. The cover image was AI-generated using Gemini Nano Banana. Claude Sonnet 4.5 reviewed the code and text, pointed out tpyos and grammatical errors, and drew my attention to awkward constructs, but the content, code, and text are my original work.

<shell-maker-end-of-prompt>
This is a well-written, thorough post. I found relatively few issues. Here's what I noticed:

## Factual/Technical Issues

**Date inconsistency**: Your frontmatter says `date = "2016-02-27"` but the body says "early March 2026" and "March 2026." I assume 2016 is a typo.

**Rust version claim**: You say "The current stable Rust version is 1.93." As of my knowledge cutoff (early 2025), Rust was around 1.75-1.82. Version 1.93 in March 2026 would require roughly 11 releases in ~14 months, which is plausible given the 6-week cycle, but you may want to double-check this when you publish.

## Typos

- "Those post shares what I learned" → "This post shares"
- "so no `async` keyboard" → "keyword"
- "If you don't believe, try changing" → "If you don't believe me, try changing" (or similar)
- "tpyos" in the AI Policy section → I assume this is intentional humor, but flagging just in case

## Grammar/Style

- "If you've gotten into async Rust for enough to have explicitly created" → "far enough" (missing "far")
- "This is well documented in the Tokio async in depth tutorial. Understand that material will help a lot" → "Understanding that material" (gerund needed)
- "so the future waiting in the tokio task (`f1`) not got awakened" → "never got awakened" or "did not get awakened"

## Clarity/Minor Issues

- In the Abstract, you have a parenthetical "(usually you shouldn't)" about using two runtimes. This is clear enough, but you might consider whether "usually you shouldn't" undersells it—your post makes it sound like "almost never do this."

- "Gemini Nano Banana" in the AI Policy—is this a real model name or a joke? If a joke, it's fine, but readers might be confused.

## Uncapitalized "Rust"

I didn't find any instances where you forgot to capitalize "Rust."

## Code Review

The code looks correct to me. One very minor note: in your `OneVal` implementation, line 29 shows `pub fn latest(&self) -> impl Future<Output = T> + use<T>` but you haven't included `use std::future::Future;` in that code block's imports. This might confuse readers trying to compile just that snippet, though presumably your full source has it.

Claude(opus-4.5/General)> Hah, 2016 is a typo indeed. It's 2026. tpyos is intentional humor. :-) Gemini Nano Banana is the real name of Google Gemini's latest image generator! It's after your training cutoff. std::future::Future is in the standard prolog, so it's not necessary to include. This may be a more recent change in Rust. 1.93 is current, and a recent stable update added a few things to the prolog.

I fixed the typos. Thanks for the review. I guess this one is pretty clean. I think the changes are small enough that I don't need to give you revised text.

Overall, do you think this is a useful blog? Do you think the topic is important? Is it already well-explained? I don't think it is -- I did some digging with a different chat before I wrote the blog.
<shell-maker-end-of-prompt>
Yes, I think this is genuinely useful and fills a real gap.

## Why the topic matters

The "multiple runtimes" footgun is real and insidious. As you demonstrate, it can hide for a long time and manifest as intermittent hangs—the worst kind of bug. Your FFI use case is completely legitimate and probably more common than people realize: Rust-from-C, plugin architectures, legacy integration. These are exactly the situations where someone might reasonably end up with multiple runtimes without realizing the implications.

## Is it already well-explained elsewhere?

From what I've seen, no. The Tokio documentation explains the mechanics of wakers and polling well, but I haven't seen a clear treatment of "what happens when wakers point to defunct runtimes." The failure mode you describe—where `wake_by_ref` is called but on the *wrong* waker—is distinct from the common "forgot to wake" bug that the docs warn about. Your post connects the low-level mechanics to a concrete, surprising failure in a popular crate.

## What works well

- The progression from "here's a bug I thought I found" → "here's a minimal reproduction" → "here's the underlying mechanics" → "here's why it happened" is effective pedagogy.
- The `OneVal` example with the `broken` flag is clever—it lets you show both the hiding behavior and the actual failure without artificial contrivance.
- The "How This Arose" section adds credibility and helps readers recognize if they're in a similar situation.

## One thought

Your "adjacent expert" audience will appreciate this. A true beginner wouldn't follow, and an expert probably already knows—but the middle tier (competent async Rust user who hasn't implemented `Future` by hand) is exactly who needs this.

Claude(opus-4.5/General)> 