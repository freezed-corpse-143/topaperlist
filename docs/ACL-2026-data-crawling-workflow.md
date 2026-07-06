# Data Crawling Workflow

This document describes the automated pipeline for collecting conference paper metadata (title, authors, URL, BibTeX, DOI) and producing the standardized JSONL + TXT files used in this repository.

---

## 1. Discovery

**Goal**: Identify which conferences in `README.md` have a `-` in the target year column, then check if acceptance results have been published online.

### Steps

1. Parse `README.md` tables and list all conferences where the target year is `-`.
2. For each candidate, run a web search to determine whether the accepted papers list or proceedings are already public.
3. Record a verdict:
   - **Ready**: full paper list is available, metadata can be scraped.
   - **Pending**: not yet announced.
   - **N/A**: conference does not take place that year (e.g., biennial odd-year events like ICCV, SOSP, NAACL).

### Sources to check

| Source | Typical URL pattern |
|---|---|
| Official conference website | `https://{year}.{conf}.org/program/accepted-papers/` |
| OpenReview | `https://openreview.net/group?id=...` |
| Publisher volume page | `https://aclanthology.org/volumes/{year}.{conf}-{type}/` |
| IEEE Xplore proceedings | `https://ieeexplore.ieee.org/xpl/conhome/...` |
| DBLP | `https://dblp.org/db/conf/{conf}/{conf}{year}.html` |

---

## 2. Source Parsing

**Goal**: Extract the list of accepted papers — at minimum (title, authors) — from the conference's published list.

### Common page formats

#### HTML list (ACL 2026)
- Structure: `<li><p><strong>TITLE</strong><br/><em>AUTHORS</em></p></li>`
- Parse with regex: `<p><strong>(.*?)</strong><br\s*/?>\s*<em>(.*?)</em></p>`
- Clean HTML entities (`&#39;` → `'`, `&amp;` → `&`) and strip inner tags (`<em>`, `<span>`).

#### OpenReview JSON
- API endpoint returns JSON with `title`, `authors`, `content` fields.
- Directly parseable, no regex needed.

#### Conference table
- Varied: `<table><tr><td>TITLE</td><td>AUTHORS</td></tr></table>`
- Extract by row, handle colspan/rowspan.

#### Plain text list
- One title per line, authors on following line or same line.
- Convert to structured pairs.

### Title normalization

Before matching against external databases, normalize titles:

```python
def normalize_title(t):
    t = t.lower().strip()
    t = re.sub(r'[{}]', '', t)           # Remove BibTeX braces
    t = unicodedata.normalize('NFKD', t) # ³ → 3, ₂ → 2
    t = re.sub(r'[^\w\s]', ' ', t)       # Punctuation → space
    t = re.sub(r'\s+', ' ', t).strip()
    return t
```

This handles superscripts (³ → 3), subscripts (₂ → 2), typographic quotes, and BibTeX protection braces.

---

## 3. Metadata Linking

**Goal**: Map each (title, authors) pair from the accepted list to a stable identifier that carries a URL and BibTeX.

### Linking strategies by conference ecosystem

#### ACL Anthology (ACL, EMNLP, NAACL, EACL, COLING)
- **Volume pages** list all papers with their Anthology IDs:
  - `https://aclanthology.org/volumes/{year}.acl-long/` (Long Papers)
  - `https://aclanthology.org/volumes/{year}.acl-short/` (Short Papers)
  - `https://aclanthology.org/volumes/{year}.acl-findings/` (Findings)
- Parse raw HTML: `<strong><a class=align-middle href=/({id})/>{title}</a></strong>`
- Match accepted page titles against volume titles using `normalize_title()`.
- Once matched: URL = `https://aclanthology.org/{id}/`, BibTeX = `https://aclanthology.org/{id}.bib`

**Important**: The accepted page order does NOT always match the Anthology ID numbering. Always match by title.

#### IEEE (CVPR, ICML, ICDE, INFOCOM, S&P)
- IEEE Xplore proceedings provide DOI-based links.
- BibTeX available at: `https://doi.org/{doi}` (with `text=bibtex` parameter) or direct API.
- URL pattern: `https://doi.org/10.1109/{conf}{year}.{id}`

#### ACM (SIGCOMM, SIGGRAPH, MobiCom, STOC)
- ACM DL uses DOI: `https://dl.acm.org/doi/10.1145/{number}`
- BibTeX: `https://dl.acm.org/doi/10.1145/{number}.bib`

#### Proceedings of Machine Learning Research (ICML, COLT)
- PMLR URL: `https://proceedings.mlr.press/v{volume}/`
- BibTeX: available per-paper on the PMLR page.

#### OpenReview (NeurIPS, ICLR)
- OpenReview API returns full metadata including BibTeX.
- Forum ID: `https://openreview.net/forum?id={forum_id}`
- API: `https://api.openreview.net/notes?forum={forum_id}`

### When matching fails

Three common failure modes and fixes:

| Failure | Cause | Fix |
|---|---|---|
| Title mismatch | Typo in accepted list vs. canonical title | Use fuzzy matching (shared-word scoring, min 4 common words) |
| Accepted count > volume count | Findings papers or additional tracks listed on same page | Cross-check against separate Findings/Industry page |
| Anthology ID not found | Paper may be in a different volume (e.g., `acl-short` vs `acl-long`) | Search all volumes for the same conference |

---

## 4. Batch Retrieval

**Goal**: Download BibTeX files for all matched papers efficiently.

### Concurrency

- Use `ThreadPoolExecutor` with 20 workers.
- Batches of 100–200 papers per progress checkpoint.
- Set per-request timeout (30s) to avoid hanging on unresponsive servers.

### Retry strategy

```python
def download_bib(url, retries=2):
    for attempt in range(retries):
        try:
            resp = urllib.request.urlopen(url, timeout=30)
            return resp.read().decode("utf-8")
        except (urllib.error.HTTPError, ConnectionError) as e:
            if attempt == retries - 1:
                raise
            time.sleep(1)
```

### HTTP 404 handling

When a `.bib` URL returns 404:
- The paper likely belongs to a different volume or has not been indexed yet.
- Log the Anthology ID and continue.
- Skip non-existent IDs in assembly; do not create records for them.

### Network errors

Connection resets (WinError 10054) or timeouts:
- Retry once after a short delay.
- If still failing, log the error and skip (data can be backfilled in the next incremental run).

---

## 5. Assembly & Validation

### JSONL format

Each line is a JSON object with these fields:

```json
{
  "title": "Paper Title",
  "author": "Author1, Author2, ...",
  "url": "https://aclanthology.org/{year}.{conf}.{type}.{n}/",
  "anthology_id": "{year}.{conf}.{type}.{n}",
  "bib": "@inproceedings{...\n}",
  "doi": "10.18653/v1/{year}.{conf}.{type}.{n}"
}
```

- `title`: from the accepted page (may contain BibTeX protect braces `{}`).
- `author`: comma-separated from the accepted page.
- `url`: canonical paper page.
- `anthology_id`: internal identifier (optional, ecosystem-specific).
- `bib`: raw BibTeX string.
- `doi`: extracted from BibTeX via regex `doi\s*=\s*"([^"]+)"`.

### TXT format

One paper title per line, no other fields. Example:

```
OctoTools: A Multi-Agent Framework with Extensible Tools for Complex Reasoning
No Reader Left Behind: Multi-Agent Summaries Everyone Can Understand
...
```

### Validation checks

| Check | Method |
|---|---|
| Line count consistency | `wc -l jsonl == wc -l txt` |
| Duplicate anthology IDs | `Counter(anthology_id).most_common(1)[1] > 1` |
| Duplicate titles | `len(set(titles)) == len(titles)` |
| Missing BibTeX | Filter records where `bib == ""` |
| Missing DOI | Filter records where `doi == ""` |
| All lines parse as JSON | `json.loads(line)` in a loop |
| UTF-8 validity | `open(..., encoding="utf-8")` |

### Data integrity rules

- Every record must have a non-empty `bib` field.
- Every record must have a non-empty `url` field.
- Duplicate anthology IDs must be resolved by keeping the first occurrence.
- The `txt` file must be regenerated from the deduplicated `jsonl`, not independently.

---

## 6. Incremental Maintenance

### Cache mechanism

Maintain a local dictionary of successfully downloaded `anthology_id → bib` results. On the next run:

1. Load the cache from disk.
2. For each paper whose `anthology_id` is already in the cache, skip the download.
3. For new or previously failed papers, download and add to cache.
4. Save the updated cache at the end.

This makes subsequent runs fast and resilient to interruptions.

### Recovery from partial runs

- If the script is interrupted mid-batch, partial results are still written to the output files.
- Re-running overwrites existing output; papers already in the cache are skipped.
- To force a refresh, delete the cache file.

### Backfilling unmatched papers

Papers not matched by title (typically 5-20 per conference) require manual intervention:

1. Search the Anthology or publisher site for the exact title.
2. Note the correct anthology ID or DOI.
3. Add a manual override mapping: `{accepted_index: correct_id}`.
4. Re-run the download step for those IDs only.

---

## Ecosystem Reference

| Conference | Anthology pattern | Volume types | BibTeX endpoint |
|---|---|---|---|
| ACL | `{year}.acl-{type}.{n}` | long, short, findings | `https://aclanthology.org/{id}.bib` |
| EMNLP | `{year}.emnlp-{type}.{n}` | long, short, findings | `https://aclanthology.org/{id}.bib` |
| NAACL | `{year}.naacl-{type}.{n}` | long, short | `https://aclanthology.org/{id}.bib` |
| EACL | `{year}.eacl-{type}.{n}` | long, short | `https://aclanthology.org/{id}.bib` |
| COLING | `{year}.coling-{type}.{n}` | long, short | `https://aclanthology.org/{id}.bib` |
| NeurIPS | `https://proceedings.neurips.cc/paper_files/paper/{year}/...` | — | Embedded in proceedings page |
| ICML | `https://proceedings.mlr.press/v{vol}/` | — | Per-paper page |
| CVPR | IEEE DOI | — | `https://doi.org/{doi}` with `?text=bibtex` |
| IEEE S&P | IEEE DOI | — | Same as CVPR |

---

## Glossary

| Term | Definition |
|---|---|
| Anthology ID | Unique identifier in the ACL Anthology, e.g. `2026.acl-long.42` |
| Accepted page | Conference website listing all accepted papers for a given year |
| Volume page | Publisher page listing all papers in a proceedings volume |
| Findings | Secondary track for papers that did not make the main conference but are still published |
| Fuzzy matching | Matching titles by shared word overlap when exact match fails |
