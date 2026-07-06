# IJCAI 2026 Data Crawling Workflow

This document describes the pipeline for collecting IJCAI 2026 paper metadata (title, authors, URL, BibTeX, DOI) and producing the standardized JSONL + TXT files used in this repository.

---

## 1. Discovery

**Goal**: Verify that IJCAI 2026 accepted papers are public and identify the data source.

### Status (as of 2026-07-06)

| Item | Value |
|---|---|
| Conference | IJCAI-ECAI 2026 (35th International Joint Conference on Artificial Intelligence) |
| Location | Bremen, Germany |
| Conference dates | August 19–23, 2026 |
| Accepted papers page | `https://2026.ijcai.org/accepted-papers/` |
| Total accepted papers | 953 (Main track: 713, Special tracks: 85, Survey: 45, Demo: 50) |
| Proceedings URL | `https://www.ijcai.org/proceedings/2026/` — **not yet online** |
| DBLP page | `https://dblp.org/db/conf/ijcai/ijcai2026.html` — **not yet indexed** |

### Initial state

- README marks IJCAI 2026 as `-`.
- Web search confirms the accepted list is published on the official website.
- Verdict: **Partial** — title + author available, BibTeX/DOI pending proceedings publication.

---

## 2. Source Parsing

**Goal**: Extract the list of accepted papers — title and authors — from the official accepted page.

### Page structure

The IJCAI 2026 website is a WordPress site. Papers are rendered in a single `<ol>` with 953 `<li>` items:

```html
<li>
  <div class="ij-top">
    <span class="ij-pid">#29</span>
    <div class="ij-sched">...</div>
  </div>
  <h3 class="ij-ptitle">Paper Title Here</h3>
  <div class="ij-authors">
    <span class="ij-author">First Author</span><span class="ij-sep">, </span>
    <span class="ij-author">Second Author</span><span class="ij-sep">, </span>
    ...
  </div>
</li>
```

### Extraction regex

```python
pattern = r'<h3 class="ij-ptitle">(.*?)</h3>\s*<div class="ij-authors">(.*?)</div>'
```

- Extract title from h3 content.
- Extract each author via `<span class="ij-author">(.*?)</span>` within the author div.
- Join authors with `, `.
- Clean HTML entities: `&#8217;` → `'`, `&#038;` → `&`.

### Paper count breakdown

The page includes tabs that categorize accepted papers:

| Track | Count |
|---|---|
| Main Track | 713 |
| Special Track on AI and Health | 47 |
| Special Track on AI and Robotics | 11 |
| Special Track on AI and Social Good | 52 |
| Special Track on AI4Tech | 25 |
| Special Track on Human-Centred AI | 10 |
| Survey Track | 45 |
| Demonstrations Track | 50 |
| **Total** | **953** |

All tracks are listed in a single ordered list on the same page. The tabs only filter client-side; the full list is in the DOM.

---

## 3. Metadata Linking

**Goal**: Map each (title, authors) pair to a DOI for BibTeX retrieval.

### IJCAI DOI pattern

IJCAI uses DOI prefix `10.24963/ijcai.{year}/{submission_id}`.

From existing 2025 data:

| Field | Pattern | Example |
|---|---|---|
| DOI | `10.24963/ijcai.{year}/{id}` | `10.24963/ijcai.2025/111` |
| URL | `https://doi.org/10.24963/ijcai.{year}/{id}` | `https://doi.org/10.24963/ijcai.2025/111` |
| DBLP key | `conf/ijcai/{AuthorKey}{year}` | `conf/ijcai/FengYZZL25` |
| DBLP bib | `https://dblp.org/rec/conf/ijcai/{key}.bib` | `https://dblp.org/rec/conf/ijcai/FengYZZL25.bib` |

### Current limitation

**The `{submission_id}` is NOT derivable from the page content.** The paper IDs shown on the accepted page (`#29`, `#35`, `#108`) are presentation numbers, not submission IDs. The actual mapping from title to DOI can only be obtained after:

1. Proceedings are published at `https://www.ijcai.org/proceedings/2026/`
2. Or DBLP indexes IJCAI 2026 at `https://dblp.org/db/conf/ijcai/ijcai2026.html`

### Placeholder record format

Until proceedings are published, each JSONL record stores an empty `url`, `bib`, and `doi`:

```json
{
  "title": "Frequency-Aware Augmentation and Alignment for Time Series Contrastive Learning",
  "author": "Yusen Liu, Zhichen Lai, Hua Lu, Xu Cheng, Xiufeng Liu, Huan Huo",
  "url": "",
  "bib": "",
  "doi": ""
}
```

---

## 4. Batch Retrieval (Post-Proceedings)

**Goal**: Backfill BibTeX and DOI after proceedings go live. This section describes the planned approach.

### Proceedings page format

The IJCAI proceedings page (`https://www.ijcai.org/proceedings/{year}/`) lists papers with:

```
Paper Title
Author1, Author2, ...
(PDF | Details)
```

- PDF link: `{number}.pdf` (zero-padded to 4 digits, e.g. `0001.pdf`)
- Details link: `/proceedings/{year}/{number}`
- The `{number}` in the details link IS the submission ID used in the DOI: `10.24963/ijcai.{year}/{number}`

### Backfill procedure

```python
# Step 1: Parse proceedings page to build title -> submission_id map
# Step 2: For each record, match title to get submission_id
# Step 3: Construct DOI = f"10.24963/ijcai.2026/{submission_id}"
# Step 4: Construct URL = f"https://doi.org/10.24963/ijcai.2026/{submission_id}"
# Step 5: Construct DBLP bib URL (if available)
# Step 6: Download bib, update record, write jsonl
```

### Matching strategy

- Use the same `normalize_title()` function as in the ACL workflow.
- The proceeding page lists papers in the same order as the accepted page (presentation order), but the submission IDs are not sequential.
- Match by title, not by position.

### Alternative: DBLP batch download

If DBLP has indexed IJCAI 2026, the entire volume bib can be downloaded at once:

```
https://dblp.org/db/conf/ijcai/ijcai2026.bib
```

This returns a single BibTeX file containing all papers. Parse it into individual records and match by title.

---

## 5. Assembly & Validation

### JSONL format (initial)

Until backfill, records have empty url/bib/doi:

```json
{
  "title": "Paper Title",
  "author": "Author1, Author2, ...",
  "url": "",
  "bib": "",
  "doi": ""
}
```

### TXT format

One paper title per line, no other fields.

### Validation checks (post-backfill)

| Check | Method |
|---|---|
| Line count consistency | `wc -l jsonl == wc -l txt` |
| All records have DOI | Filter where `doi == ""` — should be 0 after backfill |
| All records have BibTeX | Filter where `bib == ""` — should be 0 after backfill |
| Duplicate titles | `len(set(titles)) == len(titles)` |
| All lines parse as JSON | `json.loads(line)` in a loop |
| UTF-8 validity | `open(..., encoding="utf-8")` |

### Data integrity rules

- The `txt` file must be regenerated from the deduplicated `jsonl`, not independently.
- Empty `url`/`bib`/`doi` fields are acceptable during the initial phase but must be flagged for backfill.

---

## 6. Incremental Maintenance

### Two-phase strategy

IJCAI 2026 requires a two-phase approach:

**Phase 1 (before proceedings)**
- Scrape accepted page → save titles + authors.
- Write placeholder jsonl with empty url/bib/doi.

**Phase 2 (after proceedings published)**
- Monitor `https://www.ijcai.org/proceedings/2026/` for availability.
- Download proceedings page, build title → submission_id mapping.
- Match against phase 1 records.
- Download BibTeX (via DBLP or individual DOI pages).
- Update jsonl with url, bib, doi in-place.

### Monitoring triggers

Check for proceedings availability:

```python
import urllib.request

def check_proceedings_ready():
    try:
        resp = urllib.request.urlopen("https://www.ijcai.org/proceedings/2026/", timeout=10)
        html = resp.read().decode("utf-8")
        # Look for paper list content
        return "Main Track" in html or "0001.pdf" in html
    except HTTPError:
        return False
```

### Re-running

- The script should detect existing jsonl records and only backfill empty `url`/`bib`/`doi` fields.
- Already-filled records are skipped.

---

## Ecosystem Reference

| Conference | DOI prefix | Proceedings URL | BibTeX source |
|---|---|---|---|
| IJCAI | `10.24963/ijcai.{year}` | `https://www.ijcai.org/proceedings/{year}/` | DBLP or DOI with `?format=bib` |
| AAAI | `10.1609/aaai.v{vol}` | `https://ojs.aaai.org/index.php/AAAI/issue/view/{vol}` | OJS or DBLP |
| ICML | — | `https://proceedings.mlr.press/v{vol}/` | PMLR page |
| NeurIPS | — | `https://proceedings.neurips.cc/paper_files/paper/{year}/` | OpenReview or proceedings page |

---

## Glossary

| Term | Definition |
|---|---|
| Submission ID | Internal IJCAI identifier used in the DOI path, e.g. `111` in `10.24963/ijcai.2025/111` |
| Accepted page | Conference website listing all accepted papers before proceedings publication |
| Proceedings page | Publisher page listing all papers with full text and DOIs after the conference |
| Backfill | Process of filling in previously empty fields (url, bib, doi) once data becomes available |
| DBLP | Computer science bibliography database that indexes conference proceedings and provides BibTeX |
