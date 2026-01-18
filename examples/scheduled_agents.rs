use agentropic_runtime::prelude::*;

fn main() {
    println!("=== Scheduled Agents Example ===\n");

    // Create scheduler with round-robin policy
    let policy = SchedulingPolicy::new(PolicyType::RoundRobin);
    let mut scheduler = Scheduler::new(policy);

    println!("Scheduler created:");
    println!("  Policy: {:?}", scheduler.policy().policy_type());

    // Create tasks
    let agent1 = AgentId::new();
    let agent2 = AgentId::new();
    let agent3 = AgentId::new();

    let task1 = Task::new(agent1, 1);
    let task2 = Task::new(agent2, 2);
    let task3 = Task::new(agent3, 3);

    // Add tasks to queue
    scheduler.queue_mut().push(task1);
    scheduler.queue_mut().push(task2);
    scheduler.queue_mut().push(task3);

    println!("\n✓ Added {} tasks to queue", scheduler.queue().len());

    // Process tasks
    println!("\nProcessing tasks:");
    while let Some(task) = scheduler.queue_mut().pop() {
        println!(
            "  Executing task - Agent: {}, Priority: {}",
            task.agent_id(),
            task.priority()
        );
    }

    println!("\n✓ All tasks processed");
    println!("  Remaining tasks: {}", scheduler.queue().len());

    // Create different schedulers
    println!("\n--- Different Scheduling Policies ---");

    let fair_share = FairShareScheduler::new(1.5);
    println!("Fair Share Scheduler - Shares: {}", fair_share.shares());

    let priority = PriorityScheduler::new(5);
    println!("Priority Scheduler - Levels: {}", priority.levels());

    let round_robin = RoundRobinScheduler::default();
    println!(
        "Round Robin Scheduler - Time slice: {:?}",
        round_robin.time_slice()
    );
}
