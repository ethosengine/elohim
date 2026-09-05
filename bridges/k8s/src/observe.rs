//! Prometheus evidence folded into the operator-owned capacity projection.
use serde_json::{json, Value};
use std::{collections::BTreeMap, fmt, time::Duration};

pub const QUERIES: [&str; 3] = [
    "sum by (node) (kube_node_status_allocatable{resource=\"cpu\"})",
    "sum by (node) (kube_node_status_allocatable{resource=\"memory\"})",
    "kube_node_status_condition{condition=\"Ready\",status=\"true\"}",
];
const MIB: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObserveRefusal(pub String);
impl fmt::Display for ObserveRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ObserveRefusal {}
type Result<T> = std::result::Result<T, ObserveRefusal>;
fn refuse(message: impl Into<String>) -> ObserveRefusal {
    ObserveRefusal(message.into())
}

#[derive(Debug, Clone)]
pub struct Observation {
    pub cpu_m: BTreeMap<String, u64>,
    pub memory_bytes: BTreeMap<String, u64>,
    pub ready: BTreeMap<String, bool>,
    pub timestamp: String,
    pub host: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldChange {
    /// JSON pointer into the ledger, suitable for independent replay of the diff.
    pub field: String,
    pub old: Value,
    pub new: Value,
}

fn samples(
    response: &Value,
    label: &str,
    scale: f64,
    condition: bool,
) -> Result<BTreeMap<String, u64>> {
    if response["status"] != "success" {
        return Err(refuse(format!("Prometheus {label}: status is not success")));
    }
    if response["data"]["resultType"] != "vector" {
        return Err(refuse(format!("Prometheus {label}: expected vector")));
    }
    let rows = response["data"]["result"]
        .as_array()
        .ok_or_else(|| refuse(format!("Prometheus {label}: missing result")))?;
    let mut values = BTreeMap::new();
    for row in rows {
        let node = row["metric"]["node"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| refuse(format!("Prometheus {label}: missing node label")))?;
        let value = row["value"][1]
            .as_str()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(f64::NAN);
        let scaled = value * scale;
        if !scaled.is_finite()
            || scaled < 0.0
            || scaled >= u64::MAX as f64
            || (!condition && scaled < 1.0)
            || (condition && value != 0.0 && value != 1.0)
            || scaled.fract() != 0.0
        {
            return Err(refuse(format!("{node}: invalid {label} value {value}")));
        }
        if values.insert(node.to_owned(), scaled as u64).is_some() {
            return Err(refuse(format!("{node}: duplicate {label} sample")));
        }
    }
    Ok(values)
}

impl Observation {
    /// Parse canned or fetched responses identically; no network or clock reads here.
    pub fn from_responses(
        cpu: &Value,
        memory: &Value,
        ready: &Value,
        timestamp: String,
        host: String,
    ) -> Result<Self> {
        chrono::DateTime::parse_from_rfc3339(&timestamp)
            .map_err(|_| refuse("invalid observation timestamp"))?;
        Ok(Self {
            cpu_m: samples(cpu, "allocatable CPU", 1000.0, false)?,
            memory_bytes: samples(memory, "allocatable memory", 1.0, false)?,
            ready: samples(ready, "Ready condition", 1.0, true)?
                .into_iter()
                .map(|(k, v)| (k, v == 1))
                .collect(),
            timestamp,
            host,
        })
    }

    pub fn fetch(base: &str) -> crate::Result<Self> {
        let mut url = reqwest::Url::parse(base)?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err("PROMETHEUS_URL must use HTTP or HTTPS".into());
        }
        let host = url
            .host_str()
            .ok_or("PROMETHEUS_URL requires a host")?
            .to_owned();
        url.set_path(&format!(
            "{}/api/v1/query",
            url.path().trim_end_matches('/')
        ));
        url.set_query(None);
        url.set_fragment(None);
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let mut responses = Vec::new();
        for query in QUERIES {
            let response = client
                .get(url.clone())
                .query(&[("query", query)])
                .send()
                .map_err(|e| e.without_url())?
                .error_for_status()
                .map_err(|e| e.without_url())?;
            responses.push(response.json::<Value>().map_err(|e| e.without_url())?);
        }
        Ok(Self::from_responses(
            &responses[0],
            &responses[1],
            &responses[2],
            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            host,
        )?)
    }

    pub fn warnings(&self, ledger: &Value) -> Result<Vec<String>> {
        let known = known_nodes(ledger)?;
        let names: std::collections::BTreeSet<_> = self
            .cpu_m
            .keys()
            .chain(self.memory_bytes.keys())
            .chain(self.ready.keys())
            .collect();
        Ok(names
            .into_iter()
            .filter(|name| !known.contains(name))
            .map(|name| {
                format!("warning: unknown node {name}; not added (operator promotion required)")
            })
            .collect())
    }
}

fn known_nodes(ledger: &Value) -> Result<Vec<String>> {
    let groups = ledger["cluster"]["nodeTypes"]
        .as_object()
        .ok_or_else(|| refuse("ledger cluster.nodeTypes missing"))?;
    let mut names = Vec::new();
    for group in groups.values() {
        for node in group["nodes"]
            .as_array()
            .ok_or_else(|| refuse("ledger nodes missing"))?
        {
            let name = node["name"]
                .as_str()
                .ok_or_else(|| refuse("ledger node name missing"))?
                .to_owned();
            if names.contains(&name) {
                return Err(refuse(format!("{name}: duplicate ledger node")));
            }
            names.push(name);
        }
    }
    if names.is_empty() {
        return Err(refuse("ledger has no known nodes"));
    }
    Ok(names)
}
fn add(total: &mut u64, value: u64, node: &str) -> Result<()> {
    *total = total
        .checked_add(value)
        .ok_or_else(|| refuse(format!("{node}: capacity overflow")))?;
    Ok(())
}
fn changes(old: &Value, new: &Value, path: &str, out: &mut Vec<FieldChange>) {
    if let (Some(a), Some(b)) = (old.as_object(), new.as_object()) {
        for (key, value) in b {
            changes(
                a.get(key).unwrap_or(&Value::Null),
                value,
                &format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
                out,
            );
        }
    } else if let (Some(a), Some(b)) = (old.as_array(), new.as_array()) {
        if a.len() == b.len() {
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                changes(a, b, &format!("{path}/{i}"), out);
            }
        } else if old != new {
            out.push(FieldChange {
                field: path.into(),
                old: old.clone(),
                new: new.clone(),
            });
        }
    } else if old != new {
        out.push(FieldChange {
            field: path.into(),
            old: old.clone(),
            new: new.clone(),
        });
    }
}

/// All validation happens on a copy: any refusal leaves the caller's ledger untouched.
/// Aggregate memory is summed in bytes before flooring to Mi; curated fields are retained.
pub fn fold_observation(ledger: &mut Value, obs: &Observation) -> Result<Vec<FieldChange>> {
    let names = known_nodes(ledger)?;
    for name in &names {
        for (label, values, minimum) in [("CPU", &obs.cpu_m, 1), ("memory", &obs.memory_bytes, MIB)]
        {
            let value = values
                .get(name)
                .ok_or_else(|| refuse(format!("{name}: missing allocatable {label}")))?;
            if *value < minimum {
                return Err(refuse(format!("{name}: zero allocatable {label}")));
            }
        }
        if !obs.ready.contains_key(name) {
            return Err(refuse(format!("{name}: missing Ready condition")));
        }
    }
    let mut next = ledger.clone();
    let mut cpu = 0;
    let mut memory = 0;
    for group in next["cluster"]["nodeTypes"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        let mut group_cpu = 0;
        let mut group_memory = 0;
        for node in group["nodes"].as_array_mut().unwrap() {
            let name = node["name"].as_str().unwrap().to_owned();
            add(&mut group_cpu, obs.cpu_m[&name], &name)?;
            add(&mut group_memory, obs.memory_bytes[&name], &name)?;
            node["ready"] = json!(obs.ready[&name]);
            node["allocatable"]["cpu_m"] = json!(obs.cpu_m[&name]);
            node["allocatable"]["memory_Mi"] = json!(obs.memory_bytes[&name] / MIB);
        }
        group["totals"]["cpu_m"] = json!(group_cpu);
        group["totals"]["memory_Mi"] = json!(group_memory / MIB);
        add(&mut cpu, group_cpu, "cluster")?;
        add(&mut memory, group_memory, "cluster")?;
    }
    next["cluster"]["nodeCount"] = json!(names.len());
    next["cluster"]["readyNodeCount"] = json!(names.iter().filter(|n| obs.ready[*n]).count());
    next["cluster"]["notReadyNodes"] =
        json!(names.iter().filter(|n| !obs.ready[*n]).collect::<Vec<_>>());
    for (field, total) in [("cpu_m", cpu), ("memory_Mi", memory / MIB)] {
        let committed = ledger["cluster"]["totalCommitted"][field]
            .as_u64()
            .ok_or_else(|| refuse(format!("ledger totalCommitted.{field} missing")))?;
        let headroom = total.checked_sub(committed).ok_or_else(|| {
            refuse(format!(
                "cluster {field}: allocatable below retained commitments"
            ))
        })?;
        next["cluster"]["totalAllocatable"][field] = json!(total);
        next["cluster"]["totalHeadroom"][field] = json!(headroom);
    }
    // Only a pre-existing status note changes; measured utilization and notes are never refreshed.
    for name in &names {
        if next["cluster"]["actuals"][name].get("status").is_some() {
            next["cluster"]["actuals"][name]["status"] = json!(format!(
                "{} (observed {} via Prometheus); utilization not re-sampled in this promotion",
                if obs.ready[name] { "Ready" } else { "NotReady" },
                obs.timestamp
            ));
        }
    }
    let previous = ledger["snapshotTimestamp"]
        .as_str()
        .ok_or_else(|| refuse("ledger snapshotTimestamp missing"))?;
    next["snapshotTimestamp"] = json!(obs.timestamp);
    next["snapshotMethod"] = json!(format!("k8s-bridge observe from Prometheus host {} using kube_node_status_allocatable (CPU cores and memory bytes) and kube_node_status_condition (Ready) at {}; commitments/actuals/ephemeral storage retained from {}.", obs.host, obs.timestamp, previous));
    let mut diff = Vec::new();
    changes(ledger, &next, "", &mut diff);
    *ledger = next;
    Ok(diff)
}
