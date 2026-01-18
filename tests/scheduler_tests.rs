use agentropic_runtime::prelude::*;

#[test]
fn create_scheduler() {
    let policy = SchedulingPolicy::new(PolicyType::RoundRobin);
    let scheduler = Scheduler::new(policy);

    assert_eq!(scheduler.policy().policy_type(), PolicyType::RoundRobin);
    assert!(scheduler.queue().is_empty());
}

#[test]
fn task_queue_operations() {
    let mut queue = TaskQueue::new();

    let task1 = Task::new(AgentId::new(), 1);
    let task2 = Task::new(AgentId::new(), 2);

    queue.push(task1);
    queue.push(task2);

    assert_eq!(queue.len(), 2);
    assert!(!queue.is_empty());

    let task = queue.pop().unwrap();
    assert_eq!(task.priority(), 1);
    assert_eq!(queue.len(), 1);
}

#[test]
fn fair_share_scheduler() {
    let scheduler = FairShareScheduler::new(1.5);
    assert_eq!(scheduler.shares(), 1.5);
}

#[test]
fn priority_scheduler() {
    let scheduler = PriorityScheduler::new(10);
    assert_eq!(scheduler.levels(), 10);
}

#[test]
fn round_robin_scheduler() {
    let mut scheduler = RoundRobinScheduler::default();

    assert_eq!(scheduler.current_index(), 0);
    scheduler.next();
    assert_eq!(scheduler.current_index(), 1);
    scheduler.reset();
    assert_eq!(scheduler.current_index(), 0);
}
