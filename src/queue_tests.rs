#[cfg(test)]
mod queue_tests {
    use crate::queue::{Task, WorkQueue};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};
    use std::{sync, thread, time};

    const DELAY: time::Duration = Duration::from_millis(200);
    const CORRECT_RESULT: i64 = 123456;

    #[derive(Debug)]
    struct TestTask {
        counter: sync::Arc<AtomicUsize>, // safely-shared counter, to track number of tasks run
    }
    impl Task for TestTask {
        type Output = i64;
        fn run(&self) -> Option<i64> {
            thread::sleep(DELAY);
            _ = &self.counter.fetch_add(1, Ordering::SeqCst);
            Some(CORRECT_RESULT)
        }
    }

    #[test]
    // Test that the work queue can do jobs and get correct results back.
    fn basics() {
        let n_threads: usize = 2;
        let n_tasks: usize = 20;
        let n_run = sync::Arc::<AtomicUsize>::new(0.into());

        let mut q = WorkQueue::<TestTask>::new(n_threads);

        for _ in 0..n_tasks {
            q.enqueue(TestTask {
                counter: n_run.clone(),
            })
            .unwrap();
        }

        // If <n_tasks results are returned, this will deadlock.
        for _ in 0..n_tasks {
            let r = q.recv();
            assert_eq!(r, CORRECT_RESULT);
        }

        // Give leftover workers time to complete, but there shouldn't be any.
        thread::sleep(3 * DELAY);

        // No more results should be produced, so try_recv is expected to return Err
        let r = q.try_recv();
        assert!(r.is_err());

        // Make sure the correct number of tasks has actually been run.
        let n_run_ref_count = sync::Arc::strong_count(&n_run);
        assert_eq!(n_run_ref_count, 1);
        let final_n_run = sync::Arc::try_unwrap(n_run).unwrap().load(Ordering::SeqCst);
        assert_eq!(final_n_run, n_tasks);
    }

    #[test]
    // Test that the work queue is actually doing things concurrently in the right way.
    fn concurrently() {
        let n_threads: usize = 4; // Should be easy to parallelize the .Sleep call, regardless of number of cores.
        let n_tasks: usize = 20;
        let n_run = sync::Arc::<AtomicUsize>::new(0.into()); // not used in this test

        let mut q = WorkQueue::<TestTask>::new(n_threads);

        for _ in 0..n_tasks {
            q.enqueue(TestTask {
                counter: n_run.clone(),
            })
            .unwrap();
        }

        // Time how long it takes to get all of the results out.
        let start = Instant::now();
        for _ in 0..n_tasks {
            let r = q.recv();
            assert_eq!(r, CORRECT_RESULT);
        }
        let end = Instant::now();

        // Time taken should be close to what we expect
        let time_taken = end.duration_since(start).as_millis();
        let target_time = DELAY.as_millis() * (n_tasks / n_threads) as u128;
        assert!(time_taken as f64 <= (target_time as f64) * 1.3,
                "Queue appears to not be running tasks concurrently: n_workers tasks should be happening in parallel. Took {} ms, expected {} ms,", time_taken, target_time
        );

        assert!(time_taken as f64 > (target_time as f64) * 0.9,
                "Queue appears to be running too concurrently: it should only start n_workers concurrent tasks. Took {} ms, expected {} ms,", time_taken, target_time
        );
    }

    #[test]
    // Test that the work queue stops processing jobs when asked to do so.
    fn stop() {
        let n_threads: usize = 4;
        let n_tasks: usize = 50;
        let n_run = sync::Arc::<AtomicUsize>::new(0.into());

        let mut q = WorkQueue::<TestTask>::new(n_threads);

        for _ in 0..n_tasks {
            q.enqueue(TestTask {
                counter: n_run.clone(),
            })
            .unwrap();
        }

        let mut n_results: usize = 0;
        for r in q.iter() {
            assert_eq!(r, CORRECT_RESULT);
            n_results += 1;
            if n_results > n_threads {
                // Pretend n_threads tasks give result that don't mean "complete". (*)
                q.shutdown(); // After that, tell the queue we're done and to stop processing tasks;
                break; // and we're done.
            }
        }

        // give workers long enough to do whatever they're going to do
        thread::sleep((2 * n_tasks / n_threads) as u32 * DELAY);

        // We expect:
        // n_threads tasks for "incomplete" work (* above);
        // n_threads running when we send the shutdown signal;
        // up to n_threads started while shutting down.
        let final_n_run = sync::Arc::try_unwrap(n_run).unwrap().load(Ordering::SeqCst);
        assert!(
            final_n_run <= n_threads * 3,
            "too many tasks executed: {}",
            final_n_run
        );
        assert!(
            final_n_run >= n_threads * 2,
            "not enough tasks executed: {}",
            final_n_run
        );
    }

    #[test]
    // Test that checks that threads aren't being leaked
    fn thread_leak() {
        let n_threads: usize = 10;
        let n_tasks: usize = 4000;
        let n_run = sync::Arc::<AtomicUsize>::new(0.into());

        let mut q = WorkQueue::<TestTask>::new(n_threads);
        for _ in 0..n_tasks {
            q.enqueue(TestTask {
                counter: n_run.clone(),
            })
            .unwrap();
        }
        q.shutdown();

        // After .shutdown returns, we expect all worker threads to have been joined: if anybody is still working, it's a problem.
        let before: usize = (*n_run).load(Ordering::SeqCst);
        thread::sleep(5 * DELAY);
        let after: usize = (*n_run).load(Ordering::SeqCst);
        assert_eq!(
            before, after,
            "work continued after .shutdown(): threads were leaked because they weren't joined"
        );
    }
}
