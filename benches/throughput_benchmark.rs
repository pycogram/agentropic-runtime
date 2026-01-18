use agentropic_runtime::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn scheduler_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_throughput");

    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(format!("tasks_{}", size), size, |b, &size| {
            b.iter(|| {
                let policy = SchedulingPolicy::new(PolicyType::RoundRobin);
                let mut scheduler = Scheduler::new(policy);

                // Push tasks
                for _ in 0..size {
                    let task = Task::new(AgentId::new(), 1);
                    scheduler.queue_mut().push(black_box(task));
                }

                // Pop all tasks
                while scheduler.queue_mut().pop().is_some() {}
            });
        });
    }

    group.finish();
}

fn supervisor_operations(c: &mut Criterion) {
    c.bench_function("supervise_1000_agents", |b| {
        b.iter(|| {
            let mut supervisor = Supervisor::new("bench");

            for _ in 0..1000 {
                let agent_id = AgentId::new();
                let policy = RestartPolicy::new(RestartStrategy::OnFailure);
                supervisor.supervise(black_box(agent_id), black_box(policy));
            }
        });
    });
}

fn health_check_operations(c: &mut Criterion) {
    c.bench_function("health_checks_10000", |b| {
        b.iter(|| {
            let mut health = HealthCheck::new();

            for i in 0..10000 {
                if i % 2 == 0 {
                    health.record_healthy();
                } else {
                    health.record_unhealthy();
                }
                black_box(health.is_healthy());
            }
        });
    });
}

criterion_group!(
    benches,
    scheduler_throughput,
    supervisor_operations,
    health_check_operations
);
criterion_main!(benches);
