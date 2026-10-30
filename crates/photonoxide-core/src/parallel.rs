//! Independent work side by side on rayon's threads, collected in order: what a sweep over
//! wavelengths, widths or grids needs when each point is its own problem.
//!
//! The results come back in the items' order, so a caller that prints or records them after
//! gets the same output as a loop over the items, whatever the number of threads, when each
//! item's work doesn't depend on the threads (photonoxide's solvers don't). `RAYON_NUM_THREADS`
//! sets the threads.

use rayon::prelude::*;

/// `f` applied to each of `items`, side by side on rayon's threads, the results in the items'
/// order.
///
/// ```
/// let squares = photonoxide::parallel::map_in_order(&[1, 2, 3], |x| x * x);
/// assert_eq!(squares, [1, 4, 9]);
/// ```
pub fn map_in_order<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync + Send) -> Vec<R> {
    items.par_iter().map(f).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_results_come_in_the_items_order_on_any_number_of_threads() {
        let items: Vec<u64> = (0..1000).collect();
        let slow =
            |&x: &u64| (0..(x % 17) * 1000).fold(x, |a, b| a.wrapping_mul(31).wrapping_add(b));
        let expected: Vec<u64> = items.iter().map(slow).collect();
        for threads in [1, 3, 8] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
            assert_eq!(pool.install(|| map_in_order(&items, slow)), expected);
        }
    }
}
