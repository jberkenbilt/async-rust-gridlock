use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

// "Note n" comments refer to notes in the blog post's text.

// Note 1: basic OneVal type
#[derive(Default, Clone)]
pub struct OneVal<T> {
    state: Arc<Mutex<OneValState<T>>>,
}

// Note 2: helper structs for State and Future
#[derive(Default)]
struct OneValState<T> {
    val: Option<T>,
    // Keeping track of just one Waker in state shared across multiple
    // futures is not the best idea, but it's illustrative for the
    // blog post.
    waker: Option<Waker>,
}
struct OneValFut<T> {
    state: Arc<Mutex<OneValState<T>>>,
}

impl<T> OneVal<T> {
    pub fn provide(&self, val: T) {
        // Note 3: `provide` delegates
        self.state.lock().unwrap().provide(val, false);
    }

    // Even though there is no `async` keyword, this is an async
    // function because it returns a future.
    pub fn latest(&self) -> impl Future<Output = T> + use<T> {
        // Note 4: `latest` is async by explicitly returning a Future
        OneValFut {
            state: self.state.clone(),
        }
    }
}

impl<T> OneValState<T> {
    fn provide(&mut self, val: T, broken: bool) {
        // Note 5: the actual `provide` with hook to intentional break
        self.val = Some(val);
        // The `broken` parameter enables us to intentionally "forget"
        // to trigger the waker. This enables us to show when it
        // matters and when it doesn't.
        if !broken && let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref();
        }
    }
}

// Note 6: Future trait implementation
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
                // If we don't have a value, stash the waker for our
                // current context so the task can be awakened by
                // something that may be in a different task. See
                // https://tokio.rs/tokio/tutorial/async for an
                // explanation. This overwrites any previously stored
                // waker, which is probably undesirable, but we do it
                // for illustrative purposes. This is discussed in the
                // text.
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future;
    use std::pin::pin;
    use std::thread;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic() {
        let v1: OneVal<i32> = Default::default();
        // Get a reusable future.
        let mut f = pin!(v1.latest());
        // Before a value is provided, the future is not ready. An
        // await here would block.
        assert!(future::poll_immediate(&mut f).await.is_none());
        // Provide multiple values. Only the latest one wins.
        v1.provide(3);
        v1.provide(4);
        assert_eq!(f.await, 4);
        // We can clone OneVal. Any OneVal can see a value provided to
        // any of its clones.
        let v2 = v1.clone();
        v1.provide(5);
        assert_eq!(v2.latest().await, 5);
        v2.provide(6);
        assert_eq!(v1.latest().await, 6);
    }

    #[tokio::test]
    async fn test_broken1() {
        let v1: OneVal<i32> = Default::default();
        let mut f = pin!(v1.latest());
        assert!(future::poll_immediate(&mut f).await.is_none());
        v1.state.lock().unwrap().provide(4, true);
        // This still works even though we forgot to wake since await
        // explicitly polls the future.
        assert_eq!(f.await, 4);
    }

    #[tokio::test]
    async fn test_broken2() {
        let v1: OneVal<i32> = Default::default();
        let v1_clone = v1.clone();
        // Poll a future in a background task. The future only gets
        // polled when "awake."
        let mut h = pin!(tokio::spawn(v1_clone.latest()));
        // Yield to the runtime so the background task will be able to
        // poll at least once.
        tokio::time::sleep(Duration::from_millis(10)).await;
        // Now when we provide a value but forget to wake the task,
        // the background task won't poll the future.
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

    #[tokio::test(flavor = "current_thread")]
    async fn test_two_runtimes() {
        let v1: OneVal<i32> = Default::default();
        let f1 = v1.latest();
        // #1: Spawn a task to poll the future in the background.
        // Yield to the runtime so the task can poll at least once
        // when there is no value.
        let h = tokio::spawn(f1);
        tokio::time::sleep(Duration::from_millis(10)).await;
        // #2: Spawn an OS thread that polls in a separate runtime and
        // never gets a `Ready` value. This causes the cached waker to
        // have the context of the separate runtime. Wait for the
        // background task to exit before resuming so we can ensure
        // that the runtime is gone.
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
        // #3: Provide a value. This calls wake_by_ref using a waker
        // that points to the now defunct runtime created in the
        // background OS thread, so the future in the tokio task never
        // wakes up.
        v1.provide(12);
        assert!(
            tokio::time::timeout(Duration::from_millis(100), h)
                .await
                .is_err()
        );
    }
}
