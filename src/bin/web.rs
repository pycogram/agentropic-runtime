use agentropic_cognition::Rule;
use agentropic_runtime::CognitiveAgent;
use axum::{extract::State, http::StatusCode, response::Html, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Deserialize)]
struct AskRequest {
    question: String,
}

#[derive(Serialize)]
struct AskResponse {
    answer: String,
    source: String,
    beliefs: usize,
}

type SharedAgent = Arc<Mutex<CognitiveAgent>>;

async fn ask_handler(
    State(agent): State<SharedAgent>,
    Json(req): Json<AskRequest>,
) -> Result<Json<AskResponse>, StatusCode> {
    let mut agent = agent.lock().await;
    let (answer, source) = agent.think(&req.question).await;
    let beliefs = agent.belief_count();
    Ok(Json(AskResponse {
        answer,
        source: source.to_string(),
        beliefs,
    }))
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../../static/index.html"))
}

fn add_rules(agent: &mut CognitiveAgent) {
    // -- Greetings (one rule per word = 100% match on single word) --
    agent.add_rule(Rule::new("greeting:hello").with_condition("hello").with_conclusion("greeting"));
    agent.add_rule(Rule::new("greeting:hi").with_condition("hi").with_conclusion("greeting"));
    agent.add_rule(Rule::new("greeting:hey").with_condition("hey").with_conclusion("greeting"));
    agent.add_rule(Rule::new("greeting:sup").with_condition("sup").with_conclusion("greeting"));
    agent.add_rule(Rule::new("greeting:greetings").with_condition("greetings").with_conclusion("greeting"));
    agent.add_rule(Rule::new("greeting:greeting").with_condition("greeting").with_conclusion("greeting"));

    // -- What is Agentropic (general) --
    agent.add_rule(Rule::new("topic:what_is")
        .with_condition("what").with_condition("is").with_condition("agentropic")
        .with_conclusion("what_is_agentropic"));

    // -- Layman / simple explanation --
    agent.add_rule(Rule::new("topic:layman")
        .with_condition("simple").with_condition("layman").with_condition("explain")
        .with_condition("eli5").with_condition("plain").with_condition("easy")
        .with_conclusion("layman"));

    // -- Examples / what can you build --
    agent.add_rule(Rule::new("topic:examples")
        .with_condition("example").with_condition("build").with_condition("create")
        .with_condition("make").with_condition("use").with_condition("application")
        .with_condition("project").with_condition("could")
        .with_conclusion("examples"));

    // -- How it works --
    agent.add_rule(Rule::new("topic:how_it_works")
        .with_condition("how").with_condition("work").with_condition("does")
        .with_conclusion("how_it_works"));

    // -- Getting started --
    agent.add_rule(Rule::new("topic:getting_started")
        .with_condition("start").with_condition("begin").with_condition("setup")
        .with_condition("install").with_condition("quickstart").with_condition("tutorial")
        .with_conclusion("getting_started"));

    // -- Crates / architecture --
    agent.add_rule(Rule::new("topic:crates")
        .with_condition("crate").with_condition("module").with_condition("package")
        .with_condition("architecture").with_condition("structure")
        .with_conclusion("crates"));

    // -- Patterns (general) --
    agent.add_rule(Rule::new("topic:patterns")
        .with_condition("pattern").with_condition("organizational")
        .with_condition("many").with_condition("list")
        .with_conclusion("patterns"));

    // -- Individual patterns --
    agent.add_rule(Rule::new("topic:hierarchy")
        .with_condition("hierarchy").with_condition("hierarchical")
        .with_condition("command").with_condition("chain")
        .with_conclusion("hierarchy"));

    agent.add_rule(Rule::new("topic:swarm")
        .with_condition("swarm").with_condition("flock").with_condition("decentralized")
        .with_conclusion("swarm"));

    agent.add_rule(Rule::new("topic:coalition")
        .with_condition("coalition").with_condition("alliance").with_condition("temporary")
        .with_conclusion("coalition"));

    agent.add_rule(Rule::new("topic:market")
        .with_condition("market").with_condition("auction").with_condition("bid")
        .with_conclusion("market"));

    agent.add_rule(Rule::new("topic:federation")
        .with_condition("federation").with_condition("vote").with_condition("voting")
        .with_condition("governance")
        .with_conclusion("federation"));

    agent.add_rule(Rule::new("topic:team")
        .with_condition("team").with_condition("role").with_condition("leader")
        .with_condition("coordinator").with_condition("executor")
        .with_conclusion("team"));

    agent.add_rule(Rule::new("topic:holarchy")
        .with_condition("holarchy").with_condition("holon").with_condition("nested")
        .with_conclusion("holarchy"));

    agent.add_rule(Rule::new("topic:blackboard")
        .with_condition("blackboard").with_condition("shared").with_condition("knowledge")
        .with_conclusion("blackboard"));

    // -- Messaging --
    agent.add_rule(Rule::new("topic:messaging")
        .with_condition("message").with_condition("router").with_condition("send")
        .with_condition("communicate").with_condition("talk")
        .with_conclusion("messaging"));

    agent.add_rule(Rule::new("topic:performatives")
        .with_condition("performative").with_condition("fipa")
        .with_condition("inform").with_condition("request")
        .with_conclusion("performatives"));

    // -- BDI / Cognition --
    agent.add_rule(Rule::new("topic:bdi")
        .with_condition("bdi").with_condition("belief").with_condition("desire")
        .with_condition("intention")
        .with_conclusion("bdi"));

    agent.add_rule(Rule::new("topic:cognition")
        .with_condition("cognition").with_condition("reasoning").with_condition("think")
        .with_condition("reason").with_condition("belief")
        .with_conclusion("cognition_crate"));

    // -- Runtime / Supervisor --
    agent.add_rule(Rule::new("topic:runtime")
        .with_condition("runtime").with_condition("run").with_condition("execute")
        .with_condition("spawn").with_condition("task")
        .with_conclusion("runtime"));

    agent.add_rule(Rule::new("topic:supervisor")
        .with_condition("supervisor").with_condition("restart").with_condition("crash")
        .with_condition("failure").with_condition("recovery")
        .with_conclusion("supervisor"));

    // -- Agent trait --
    agent.add_rule(Rule::new("topic:agent_trait")
        .with_condition("trait").with_condition("implement").with_condition("agent")
        .with_condition("interface").with_condition("method")
        .with_conclusion("agent_trait"));

    agent.add_rule(Rule::new("topic:handle_message")
        .with_condition("handle").with_condition("receive").with_condition("incoming")
        .with_conclusion("handle_message"));

    agent.add_rule(Rule::new("topic:agent_context")
        .with_condition("context").with_condition("agentcontext")
        .with_conclusion("agent_context"));

    // -- Why Rust --
    agent.add_rule(Rule::new("topic:why_rust")
        .with_condition("why").with_condition("rust").with_condition("language")
        .with_condition("performance").with_condition("fast")
        .with_conclusion("why_rust"));

    // -- Comparison --
    agent.add_rule(Rule::new("topic:comparison")
        .with_condition("compare").with_condition("vs").with_condition("versus")
        .with_condition("difference").with_condition("other").with_condition("alternative")
        .with_condition("crewai").with_condition("autogen").with_condition("langgraph")
        .with_conclusion("comparison"));

    // -- xbot --
    agent.add_rule(Rule::new("topic:xbot")
        .with_condition("xbot").with_condition("twitter").with_condition("bot")
        .with_conclusion("xbot"));

    // -- Vision --
    agent.add_rule(Rule::new("topic:vision")
        .with_condition("vision").with_condition("goal").with_condition("roadmap")
        .with_condition("future").with_condition("plan")
        .with_conclusion("vision"));

    // -- Owner / creator --
    agent.add_rule(Rule::new("topic:owner")
        .with_condition("owner").with_condition("creator").with_condition("who")
        .with_condition("made").with_condition("built").with_condition("created")
        .with_condition("founder").with_condition("author")
        .with_conclusion("owner"));

    // -- GitHub --
    agent.add_rule(Rule::new("topic:github")
        .with_condition("github").with_condition("repo").with_condition("repository")
        .with_condition("code").with_condition("source")
        .with_conclusion("github"));
    // -- Open source / license --
    agent.add_rule(Rule::new("topic:open_source")
        .with_condition("open").with_condition("source").with_condition("license")
        .with_condition("github").with_condition("free")
        .with_conclusion("open_source"));
}

#[tokio::main]
async fn main() {
    println!("Loading CognitiveAgent...");

    let mut agent = CognitiveAgent::from_config("data/beliefs.json", "data/config.json");
    add_rules(&mut agent);

    println!("  {} beliefs, {} rules loaded", agent.belief_count(), agent.rule_count());

    let shared = Arc::new(Mutex::new(agent));

    let app = Router::new()
        .route("/", get(index))
        .route("/ask", post(ask_handler))
        .with_state(shared);

    let addr = "0.0.0.0:3000";
    println!("  Server running at http://localhost:3000");
    println!("  Press Ctrl+C to stop\n");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
