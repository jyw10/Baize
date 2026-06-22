use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeBudget {
    pub soft: Duration,
    pub hard: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    pub remaining: Duration,
    pub increment: Duration,
    pub moves_to_go: Option<u32>,
}

#[must_use]
pub fn fixed_move_time(duration: Duration) -> TimeBudget {
    let millis = duration.as_millis().min(u128::from(u64::MAX)) as u64;
    TimeBudget {
        soft: Duration::from_millis(millis.saturating_mul(9) / 10),
        hard: duration,
    }
}

#[must_use]
pub fn allocate_time(clock: Clock) -> TimeBudget {
    let remaining = clock.remaining.as_millis().min(u128::from(u64::MAX)) as u64;
    let increment = clock.increment.as_millis().min(u128::from(u64::MAX)) as u64;
    let reserve = 50_u64.min(remaining / 20);
    let usable = remaining.saturating_sub(reserve);
    let moves_to_go = u64::from(clock.moves_to_go.unwrap_or(30).max(1));
    let base = usable / moves_to_go;
    let soft = base.saturating_add(increment.saturating_mul(3) / 4).min(usable);
    let hard = soft.saturating_mul(3).max(soft.saturating_add(10)).min(usable);
    TimeBudget {
        soft: Duration::from_millis(soft.min(hard)),
        hard: Duration::from_millis(hard),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_budget_has_safety_margin() {
        let budget = fixed_move_time(Duration::from_millis(1_000));
        assert_eq!(budget.soft, Duration::from_millis(900));
        assert_eq!(budget.hard, Duration::from_millis(1_000));
    }

    #[test]
    fn clock_budget_is_bounded_by_remaining_time() {
        let budget = allocate_time(Clock {
            remaining: Duration::from_millis(1_000),
            increment: Duration::from_millis(100),
            moves_to_go: Some(20),
        });
        assert!(budget.soft <= budget.hard);
        assert!(budget.hard < Duration::from_millis(1_000));
        assert!(budget.soft > Duration::ZERO);
    }
}
