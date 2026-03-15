# Issue Report Pipeline — Design

**Date:** 2026-03-15
**Status:** Approved
**Epic:** Elohim — Exception Understanding (governance surface + diagnostic pipeline)

## Vision

Linus Torvalds' biggest regret: not handling exceptions elegantly. Every system scatters failure information across stack traces, log files, stderr streams, HTTP status codes — but nobody weaves it into a coherent narrative.

The Elohim Protocol handles this differently. An error doesn't just get thrown and caught — it enters a conversation. The elohim sits between the raw chaos of failure and the human who needs to understand what happened. It correlates, it asks clarifying questions, and it curates — turning a 200-line stack trace into "the content blob for this lesson was deleted during a sync conflict." That's not exception handling. That's exception *understanding*.

The human sees a clean summary. The full diagnostic detail is attached to the report for follow-up, but never surfaced as information overload. The elohim reviews diagnostics for sensitivity before persistence — no raw paths, no PII, no internal IDs leaking into a GitHub issue.

## Architecture

### Collection Pipeline

```
Human clicks ... -> "Report Issue"
  |
  v
+---------------------------------------------+
|  GateFeedbackModal (feedbackType='report')   |
|  "What happened?"                            |
|  +-------------------------------------+    |
|  | DiagnosticCollector (runs on open)   |    |
|  |  * LoggerService.getRecentLogs()     |    |
|  |  * Current route / EPR context       |    |
|  |  * Environment (UA, Tauri version)   |    |
|  |  * GET /health snapshot              |    |
|  |  * Avodah context (if applicable)    |    |
|  +-------------------------------------+    |
|                                              |
|  User types description -> Submit            |
+---------------------+------------------------+
                      |
                      v
+---------------------------------------------+
|  Gate Evaluation (elohim conversation)       |
|                                              |
|  Elohim sees: user description               |
|             + diagnostics_raw bundle         |
|                                              |
|  May ask: "I see a 404 on content fetch --   |
|   were you trying to load a specific         |
|   lesson?" (pausePrompt)                     |
|                                              |
|  May reclassify: "This sounds like a feature |
|   request rather than a bug -- shall I file  |
|   it that way?"                              |
|                                              |
|  Produces: summary + severity                |
|          + diagnostics_safe (sanitized)      |
+---------------------+------------------------+
                      |
                      v
+---------------------------------------------+
|  POST /db/issue-reports                      |
|                                              |
|  Persisted:                                  |
|  +-- summary (elohim-authored, human-read)   |
|  +-- description (user's words)              |
|  +-- category (bug/feature-request/question) |
|  +-- severity (info/warning/error/critical)  |
|  +-- diagnostics (sanitized JSON)            |
|  +-- context_url                             |
|  +-- environment (JSON)                      |
|  +-- avodah_context (JSON)                   |
|  +-- resolution_status (open)                |
|                                              |
|  NOT persisted: diagnostics_raw              |
|  (ephemeral -- lived only in gate dialogue)  |
+---------------------+------------------------+
                      |
                      v (future, when elohim agent has compute)
+---------------------------------------------+
|  Elohim Agent (on storage sidecar)           |
|  * Queries backend logs (correlation ID)     |
|  * Reads codebase map (route -> components)  |
|  * Creates work-story in Avodah              |
|  * Files GitHub issue via API                |
|  * Links report -> story -> issue            |
|  * REA event on resolution                   |
+---------------------------------------------+
```

### Two-Tier Visibility

**Human layer:** Clean summary. "Something went wrong when I tried to load this lesson. The elohim looked into it and found the content blob was missing."

**Agent layer:** Full diagnostic bundle — logger buffer, HTTP error chain, health snapshot, correlation IDs. Attached to the report but not surfaced unless the human follows up.

The elohim reviews diagnostics for sensitivity before persistence. No stack traces with internal paths, no PII, no raw database IDs in the persisted report. The raw diagnostics are ephemeral — they exist only during the gate dialogue.

### Not Intimidating

The user clicks "Report Issue" and gets a textarea that says "What happened?" Same gentle gate dialogue they already know from comments. Diagnostic collection is silent. They just talk.

The elohim may also recognize when someone says "it would be cool if..." and reclassify from bug to feature request. Same entry point, the elohim's judgment determines the category.

### Agent Code Awareness (Future)

The diagnostic bundle carries runtime context (logs, route, health). The elohim agent — when it has compute on the household node — has the *capability* to query the codebase map, correlate routes to components, trace service chains, and produce investigative reports. This is a tool in the agent's toolbox, not payload on the wire. No component-level registration needed — the agent reads the code.

## Data Model

### Issue Reports Table

```sql
CREATE TABLE issue_reports (
    id TEXT PRIMARY KEY NOT NULL,
    human_id TEXT NOT NULL,
    summary TEXT,                    -- elohim-authored (null until agent processes)
    description TEXT NOT NULL,       -- user's words
    category TEXT NOT NULL DEFAULT 'bug',  -- bug, feature-request, question
    severity TEXT NOT NULL DEFAULT 'info', -- info, warning, error, critical
    diagnostics TEXT NOT NULL,       -- JSON (sanitized by elohim, or raw-minus-sensitive if no agent)
    context_url TEXT,
    environment TEXT,                -- JSON
    avodah_context TEXT,             -- JSON (project/story IDs if applicable)
    resolution_status TEXT NOT NULL DEFAULT 'open',  -- open, investigating, resolved, wont-fix
    linked_github_url TEXT,          -- future: elohim fills this
    linked_work_story_id TEXT,       -- future: avodah integration
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_issue_reports_human_id ON issue_reports (human_id);
CREATE INDEX idx_issue_reports_resolution_status ON issue_reports (resolution_status);
CREATE INDEX idx_issue_reports_category ON issue_reports (category);
```

### DiagnosticBundle (TypeScript, collected at modal open)

```typescript
interface DiagnosticBundle {
  logs: LogEntry[];           // filtered recent errors/warnings from LoggerService
  environment: {
    platform: 'browser' | 'tauri';
    userAgent: string;
    appVersion: string;
    storageHealth: HealthSnapshot | null;  // GET /health at collection time
  };
  context: {
    url: string;              // current route
    eprId: string | null;     // if viewing EPR content
    avodahProject: string | null;
    avodahStory: string | null;
  };
  correlationIds: string[];   // from recent log entries, for backend correlation
  collectedAt: string;        // ISO timestamp
}
```

Collected when the modal opens — captures state when the user noticed the problem.

### IssueReportView (Rust, API response)

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueReportView {
    pub id: String,
    pub human_id: String,
    pub summary: Option<String>,
    pub description: String,
    pub category: String,
    pub severity: String,
    pub diagnostics: Value,            // parsed JSON
    pub context_url: Option<String>,
    pub environment: Option<Value>,    // parsed JSON
    pub avodah_context: Option<Value>, // parsed JSON
    pub resolution_status: String,
    pub linked_github_url: Option<String>,
    pub linked_work_story_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

### CreateIssueReportInputView (Rust, API input)

```rust
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateIssueReportInputView {
    pub description: String,
    pub category: Option<String>,       // default: "bug"
    pub severity: Option<String>,       // default: "info"
    pub diagnostics: Value,             // the full bundle
    pub context_url: Option<String>,
    pub environment: Option<Value>,
    pub avodah_context: Option<Value>,
}
```

## Component Changes

### GateFeedbackTrigger — add 4th menu item

```typescript
const MENU_ITEMS: MenuItem[] = [
  { type: 'flag', label: 'Flag' },
  { type: 'challenge', label: 'Challenge' },
  { type: 'feedback', label: 'Feedback' },
  { type: 'report', label: 'Report Issue' },  // NEW
];
```

### GateFeedbackModal — report type support

- Title: `'Report Issue'`
- Placeholder: `'What happened?'`
- On open with `feedbackType='report'`: trigger `DiagnosticCollectorService.collect()`
- Pass diagnostic bundle through `contextMetadata.diagnostics`

### DiagnosticCollectorService (new, elohim pillar)

Single injectable service. Reads LoggerService buffer, Router URL, navigator info, calls `/health`. Returns `DiagnosticBundle`. No subscriptions, no state — pure collection on demand.

### StorageApiService — new methods

- `createIssueReport(input: CreateIssueReportInputView): Observable<IssueReportView>`
- `getIssueReports(filters?): Observable<IssueReportView[]>`

### Error logging consistency

`StorageApiService.handleError()` must call `LoggerService.error()` with operation name, URL, status code, and error message. This ensures HTTP failures appear in the logger buffer and are captured in diagnostic bundles.

## What's Built Now vs Future

| Piece | Sprint |
|-------|--------|
| "Report Issue" menu item on trigger | Now |
| `DiagnosticCollectorService` | Now |
| `issue_reports` table + Rust CRUD | Now |
| `IssueReportView` + TS type generation | Now |
| `StorageApiService.createIssueReport()` | Now |
| Modal wiring for report type | Now |
| Error logging consistency in handleError() | Now |
| Screenshot capture (auto via html2canvas) | Future |
| Screenshot capture (user paste/drag) | Future |
| Elohim agent backend log correlation | Future (agent compute) |
| Elohim agent codebase map awareness | Future (agent compute) |
| Avodah work-story creation from report | Future (agent compute) |
| GitHub issue creation from report | Future (agent compute) |
| Sensitivity review of diagnostics | Future (agent compute, manual review until then) |

## Integration Points

| Existing Piece | How Pipeline Uses It |
|----------------|----------------------|
| **GateFeedbackTrigger** (just merged) | Add 4th menu item, same modal pattern |
| **GateFeedbackModal** (just merged) | New feedbackType='report', triggers diagnostic collection |
| **LoggerService** (100-entry buffer, correlation IDs) | Primary log source for diagnostics |
| **StorageApiService** (catchError pattern) | Ensure errors flow to LoggerService |
| **GET /health** (blob stats, manifest count) | Backend health snapshot in bundle |
| **Avodah** (work-project, work-story content nodes) | Context capture now, work-story creation future |
| **Gate evaluation** (elohim conversation) | Elohim sees diagnostics, asks clarifying questions, produces summary |
| **REA pipeline** | Future: resolution event when fix lands |
