use std::time::Duration;

pub struct BenchmarkOutcome {
    pub elapsed: Duration,
    pub planning_time: f64,
    pub execution_time: f64,
}

pub fn summarise_outcomes(mut outcomes: Vec<BenchmarkOutcome>) -> BenchmarkOutcome {
    // remove the fastest and slowest runs to mitigate outliers
    outcomes.sort_by_key(|o| o.elapsed);
    outcomes.pop();
    outcomes.remove(0);

    // summarise the results by taking the average of the remaining runs
    let average_elapsed =
        outcomes.iter().map(|o| o.elapsed).sum::<Duration>() / (outcomes.len() as u32);
    let average_planning_time =
        outcomes.iter().map(|o| o.planning_time).sum::<f64>() / (outcomes.len() as f64);
    let average_execution_time =
        outcomes.iter().map(|o| o.execution_time).sum::<f64>() / (outcomes.len() as f64);

    BenchmarkOutcome {
        elapsed: average_elapsed,
        planning_time: average_planning_time,
        execution_time: average_execution_time,
    }
}
