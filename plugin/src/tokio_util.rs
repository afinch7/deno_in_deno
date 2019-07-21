use futures::future::FutureExt;
use std::future::Future;
use std::pin::Pin;
use tokio::runtime;

pub fn create_threadpool_runtime() -> tokio::runtime::Runtime {
  runtime::Builder::new()
    .panic_handler(|err| std::panic::resume_unwind(err))
    .build()
    .unwrap()
}

/// THIS IS A HACK AND SHOULD BE AVOIDED.
///
/// This creates a new tokio runtime, with many new threads, to execute the
/// given future. This is useful when we want to block the main runtime to
/// resolve a future without worrying that we'll use up all the threads in the
/// main runtime.
pub fn block_on<F, R, E>(future: F) -> Result<R, E>
where
  F: Send + 'static + Future<Output = Result<R, E>>,
  R: Send + 'static,
  E: Send + 'static,
{
  use std::sync::mpsc::channel;
  use std::thread;
  let (sender, receiver) = channel();
  // Create a new runtime to evaluate the future asynchronously.
  thread::spawn(move || {
    let rt = create_threadpool_runtime();
    let r = rt.block_on(future);
    sender.send(r).unwrap();
  });
  receiver.recv().unwrap()
}

pub fn spawn_on_default<F, R, E>(
  future: F,
) -> Pin<Box<dyn Future<Output = Result<R, E>> + Send>>
where
  F: Send + 'static + Future<Output = Result<R, E>>,
  R: Send + 'static,
  E: Send + 'static,
{
  use futures::channel::oneshot::channel;
  use tokio::executor::Executor;
  let (sender, receiver) = channel();
  tokio::executor::DefaultExecutor::current()
    .spawn(
      future
        .then(|result| {
          assert!(sender.send(result).is_ok());
          futures::future::ready(())
        })
        .boxed(),
    )
    .unwrap();
  receiver.map(|result| result.unwrap()).boxed()
}