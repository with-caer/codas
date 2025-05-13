//! # Unstable
//!
//! Stages are groups of data processors that read
//! data from a shared input [`Flow`] and write data
//! to one or more output [`Flow`]s.

use core::{
    future::Future,
    ops::Range,
    pin::Pin,
    task::{Context, Waker},
};

use alloc::{boxed::Box, collections::VecDeque, vec::Vec};
use codas::types::TryAsFormat;

use crate::{async_support, Error, Flow, FlowSubscriber, Flows};

/// Group of data processors sharing a [`FlowSubscriber`].
pub struct Stage<T: Flows> {
    subscriber: FlowSubscriber<T>,

    /// Set of processors to invoke during each proc.
    #[allow(clippy::type_complexity)]
    processors: Vec<Box<dyn FnMut(&mut Proc, &T) + Send + 'static>>,

    /// Stage processing context reused
    /// between processors.
    context: Proc,

    /// Maximum number of data that will be
    /// processed in a single batch
    max_procs_per_batch: usize,
}

impl<T: Flows> Stage<T> {
    /// Returns a [`Flow`] handle connected to the stage.
    pub fn flow(&self) -> Flow<T> {
        Flow {
            state: self.subscriber.flow_state.clone(),
        }
    }

    /// Adds a new processor to the stage.
    ///
    /// `proc` may accept _any_ data type `T` which the flow's
    /// data `D` can be ["tried as"](codas::types::TryAsFormat).
    /// `proc` will only be invoked for data in the flow which
    /// is successfully interperable as `D` .
    pub fn add_proc<D>(&mut self, mut proc: impl Procs<D>)
    where
        T: TryAsFormat<D>,
    {
        let proc = move |context: &mut Proc, data: &T| {
            if let Ok(data) = data.try_as_format() {
                proc.proc(context, data);
            }

            if context.remaining() == 0 {
                proc.end_of_procs();
            }
        };

        self.processors.push(Box::new(proc));
    }

    /// Invokes each processor at least once if
    /// the flow is active and data is available,
    /// returning the number of data processed.
    pub fn proc(&mut self) -> Result<u64, Error> {
        // Snapshot currently receivable sequences.
        let receivable_seqs = self.subscriber.receivable_seqs();
        assert_eq!(receivable_seqs.start, self.context.receivable_seqs.start);
        self.context.receivable_seqs = receivable_seqs;

        // Process all immediately available sequences.
        let first_receivable = self.context.receivable_seqs.start;
        let last_receivable = first_receivable + self.max_procs_per_batch as u64;
        let mut last_received = None;
        while let Some(next) = self.context.receivable_seqs.next() {
            last_received = Some(next);

            // Fetch the data off the flow.
            let data = unsafe { self.subscriber.flow_state.get(next) };

            // Invoke all processors.
            for proc in &mut self.processors {
                (proc)(&mut self.context, data)
            }

            // End processing if we hit the last sequence
            // receivable in this batch.
            if next >= last_receivable {
                break;
            }
        }

        // Poll all outstanding tasks.
        self.context.poll_tasks();

        // Progress the subscriber.
        if let Some(last) = last_received {
            self.subscriber.receive_up_to(last);
            Ok(self.context.receivable_seqs.start - first_receivable)
        } else {
            Err(Error::Ahead)
        }
    }

    /// Runs [`Self::proc`] in an infinite loop.
    ///
    /// When the flow is idle, [`async_support::yield_now`]
    /// will be invoked to temporarily yield execution back
    /// to the async runtime. Invoke [`Self::proc_loop_with_waiter`]
    /// _instead_ of this function to use a different waiter.
    pub async fn proc_loop(mut self) {
        loop {
            if self.proc().is_err() {
                async_support::yield_now().await;
            }
        }
    }

    /// Runs [`Self::proc`] in an infinite loop, calling
    /// `waiter` when the flow is idle.
    ///
    /// Calling this function with an async runtime's
    /// "native" yield function can improve throughput
    /// by ~50% or more (as compared to [`Self::proc_loop`]).
    ///
    /// On Tokio runtimes, it's highly recommended to
    /// call this function with `tokio::task::yield_now`.
    pub async fn proc_loop_with_waiter<W, Fut>(mut self, waiter: W)
    where
        W: Fn() -> Fut,
        Fut: Future<Output = ()>,
    {
        loop {
            if self.proc().is_err() {
                waiter().await;
            }
        }
    }
}

impl<T: Flows> From<FlowSubscriber<T>> for Stage<T> {
    fn from(value: FlowSubscriber<T>) -> Self {
        let max_procs_per_batch = value.flow_state.buffer.len() / 4;

        Self {
            subscriber: value,
            context: Proc::default(),
            processors: Default::default(),
            max_procs_per_batch,
        }
    }
}

/// Data processor in a [`Stage`].
pub trait Procs<D>: Send + 'static {
    /// Processes `data` within a `context`.
    fn proc(&mut self, context: &mut Proc, data: &D);

    /// Invoked after the _final_ data in a set
    /// of data has been passed to the processor.
    ///
    /// Once invoked, there is no guarantee
    /// (`proc`)[`Procs::proc`] will be invoked
    /// again in the future.
    #[inline(always)]
    fn end_of_procs(&mut self) {}
}

impl<T, D> Procs<D> for T
where
    T: FnMut(&mut Proc, &D) + Send + 'static,
{
    fn proc(&mut self, context: &mut Proc, data: &D) {
        (self)(context, data)
    }
}

/// Contextual state of processors in a [`Stage`].
pub struct Proc {
    /// Async waker used when polling [`Self::pending_tasks`].
    waker: Waker,

    /// Pending tasks spawned by [`Self::spawn`].
    pending_tasks: VecDeque<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,

    /// Range of data sequences available and _not_ yet processed.
    receivable_seqs: Range<u64>,
}

impl Proc {
    /// Returns the number of times the processor _may_
    /// be invoked after the current invocation.
    pub fn remaining(&self) -> u64 {
        self.receivable_seqs.end - self.receivable_seqs.start
    }

    /// Schedules an asynchronous task for execution.
    pub fn spawn(&mut self, task: impl Future<Output = ()> + Send + 'static) {
        let mut context = Context::from_waker(&self.waker);
        let mut pinned = Box::pin(task);
        if pinned.as_mut().poll(&mut context).is_pending() {
            self.pending_tasks.push_back(Box::pin(pinned));
        }
    }

    /// Polls every task in [`Self::pending_tasks`] once.
    fn poll_tasks(&mut self) {
        if !self.pending_tasks.is_empty() {
            let mut context = Context::from_waker(&self.waker);
            self.pending_tasks
                .retain_mut(|future| future.as_mut().poll(&mut context).is_pending());
        }
    }
}

impl Default for Proc {
    fn default() -> Self {
        Self {
            waker: async_support::noop_waker(),
            pending_tasks: VecDeque::new(),
            receivable_seqs: 0..0,
        }
    }
}

#[cfg(test)]
mod tests {

    use core::sync::atomic::Ordering;

    use portable_atomic::AtomicU64;
    use portable_atomic_util::Arc;

    use crate::Flow;

    use super::*;

    #[test]
    fn dynamic_subscribers() {
        // Create the flow with one subscriber.
        let (mut flow, [subscriber]) = Flow::<u32>::new(32);

        // Sample data published into the stage.
        let test_data = 1337;

        // Create a stage with two identical processors.
        let invocations = Arc::new(AtomicU64::new(0));
        let mut stage = Stage::from(subscriber);
        let invocations_a = invocations.clone();
        stage.add_proc(move |proc: &mut Proc, data: &u32| {
            let data = *data;
            let invocations_a = invocations_a.clone();
            proc.spawn(async move {
                assert_eq!(test_data, data);
                invocations_a.add(1, Ordering::SeqCst);
            });
            assert_eq!(0, proc.remaining());
        });
        let invocations_b = invocations.clone();
        stage.add_proc(move |proc: &mut Proc, data: &u32| {
            let data = *data;
            let invocations_b = invocations_b.clone();
            proc.spawn(async move {
                assert_eq!(test_data, data);
                invocations_b.add(1, Ordering::SeqCst);
            });
            assert_eq!(0, proc.remaining());
        });

        // Publish data and poll.
        assert_eq!(Err(Error::Ahead), stage.proc());
        flow.try_next().unwrap().publish(test_data);
        assert_eq!(Ok(1), stage.proc());
        assert_eq!(2, invocations.load(Ordering::SeqCst));
    }
}
