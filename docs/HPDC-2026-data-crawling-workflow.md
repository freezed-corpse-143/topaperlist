# HPDC 2026 Data Crawling Workflow

This document describes the pipeline for collecting HPDC 2026 paper metadata and producing the standardized JSONL + TXT files.

---

## 1. Discovery

| Item | Value |
|---|---|
| Conference | 35th ACM International Symposium on High-Performance Parallel and Distributed Computing (HPDC 2026) |
| Location | Cleveland, OH, USA |
| Conference dates | July 13–16, 2026 |
| Program page | `https://hpdc.sci.utah.edu/2026/program.html` |
| Proceedings | ACM Digital Library (post-conference) |
| DBLP page | Not yet indexed |
| Total papers extracted | 40 |

### Verdict

- Program published ahead of conference (next week).
- 12 technical sessions with 40 full papers (including 3 best paper nominees).
- Data extracted from the schedule; no dedicated "accepted papers" page.

---

## 2. Source Parsing

### Page structure

The HPDC program page is a Jekyll-generated static HTML page. Papers are listed in tables with session blocks:

```
Session X — NAME
**TITLE**

_**Authors:**_ _Author1 , _Author2 , _Author3..._
```

The markdown extraction renders papers as:

```
**TITLE**

_**Authors:**_ _Author1 , _Author2 , _Author3..._

 |
```

### Extraction method

```javascript
const md = await tab.extract('markdown');
const lines = md.split('\n');

for (let i = 0; i < lines.length; i++) {
  const m = lines[i].match(/^\*\*(.+?)\*\*\s*$/);
  if (!m) continue;
  
  const title = m[1].trim();
  // Skip session headers, logistics
  if (title.length < 10 || title.startsWith('Session ')) continue;
  
  // Authors are at i+2 with heavy markdown formatting
  let raw = (i + 2 < lines.length) ? lines[i + 2] : '';
  let authors = raw.replace(/[_*]/g, '').replace(/Authors:\s*/i, '').trim();
  
  papers.push({ title, authors });
}
```

### Author cleaning

The raw markdown has unusual formatting due to Jekyll rendering:

```
_**Authors:**_ _Zhibin Wang , _Zetao Hong , _Xue Li ...
```

The simplest cleaning approach is to strip all underscore and asterisk characters, then remove the "Authors:" prefix. This yields clean comma-separated names.

### Known issues

- Kizashi talks (short presentations) are interleaved with full paper sessions — filter by checking if title is part of a "Kizashi Talks" subsection.
- Some Best Paper Nominee annotations are part of the title, e.g., `"CARBS: ... (Best paper nominee)"` — these are kept as-is since they are part of the canonical title.
- Poster sessions and workshop-only papers must be excluded.

---

## 3. Metadata Linking

### DOI pattern

HPDC papers use ACM DOI: `10.1145/{conference_id}.{paper_id}`.

### Backfill approach

Same strategy as ISCA:
- Wait for DBLP indexing (`https://dblp.org/db/conf/hpdc/hpdc2026.html`).
- Or search ACM DL post-conference.

---

## 4. Assembly & Validation

### JSONL format (initial)

```json
{
  "title": "STAR: Decode-Phase Rescheduling for LLM Inference",
  "author": "Zhibin Wang, Zetao Hong, Xue Li, ...",
  "url": "",
  "bib": "",
  "doi": ""
}
```

### Expected counts

| Category | Count |
|---|---|
| Full papers (12 sessions) | ~40 |
| Best paper nominees | 3 |
| Kizashi talks (not included) | ~8 |

---

## 5. Backfill Strategy

- Conference is July 13–16, 2026. Proceedings typically appear on ACM DL within 1-3 months.
- DBLP indexing follows ACM DL publication.
- Monitor both for bib availability.

---

## Ecosystem Reference

| Conference | Identifier | BibTeX source |
|---|---|---|
| HPDC | `10.1145/{conf_id}.{paper_id}` | ACM DL or DBLP |
| SC | `10.1109/SC{year}` | IEEE Xplore or DBLP |
| PPoPP | `10.1145/{conf_id}.{paper_id}` | ACM DL or DBLP |
| ICPP | `10.1145/{conf_id}.{paper_id}` | ACM DL or DBLP |
