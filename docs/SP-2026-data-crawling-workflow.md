# IEEE S&P 2026 Data Crawling Workflow

This document describes the pipeline for collecting IEEE Symposium on Security and Privacy 2026 paper metadata and producing the standardized JSONL + TXT files.

---

## 1. Discovery

| Item | Value |
|---|---|
| Conference | 47th IEEE Symposium on Security and Privacy (S&P 2026) |
| Location | San Francisco, CA, USA |
| Conference dates | May 18–21, 2026 |
| Accepted papers page | `https://sp2026.ieee-security.org/accepted-papers.html` |
| Total accepted papers | 254 (across two review cycles) |
| Proceedings DOI prefix | `10.1109/SP{conf_num}.2026.{id}` (e.g., `10.1109/SP61157.2025.00193` for 2025) |
| DBLP page | `https://dblp.org/db/conf/sp/sp2026.html` — **not yet indexed** |
| IEEE Xplore | Not yet published (conference already held, proceedings pending) |

### Initial state

- README marks S&P 2026 as `-`.
- Web search confirms the accepted list is published on the official website.
- Verdict: **Partial** — 254 titles + authors available, DOI/BibTeX pending IEEE Xplore indexing.

---

## 2. Source Parsing

**Goal**: Extract all 254 accepted paper titles and authors from the official page.

### Page structure

The S&P 2026 page is a static HTML page with two sections (Cycle 1, Cycle 2). Papers are rendered as Bootstrap list-group items:

```html
<div class="list-group-item">
  <b><a data-toggle="collapse" href="#collapse-0" aria-expanded="false"
       aria-controls="collapse-0">
    Paper Title
    <span class="glyphicon glyphicon-chevron-down"></span>
  </a></b>
  <br>
  <div class="collapse authorlist" id="collapse-0">
    Author1<sup>1</sup>, Author2<sup>2</sup>, ...
    <br>
    1: Affiliation 1<br>
    2: Affiliation 2<br>
  </div>
</div>
```

### Extraction regex

```python
# Extract titles
titles = re.findall(
    r'<b><a[^>]*data-toggle="collapse"[^>]*>([^<]+)', html
)

# Extract author content from collapse divs
authors_raw = re.findall(
    r'<div class="collapse authorlist"[^>]*id="collapse-\d+"[^>]*>'
    r'(.*?)</div>\s*</div>', html, re.DOTALL
)
```

### Author cleaning

The raw author text includes affiliation numbers and institution names:

> `Jiaqian Peng1,2, Puzhuo Liu3, ..., 1: Institute of Information Engineering, 2: ...`

For the initial jsonl, the raw author text (including affiliations) is preserved as-is. When backfilling from DBLP, the canonical author list will be used instead.

### Paper count

| Cycle | Count |
|---|---|
| Cycle 1 | ~113 |
| Cycle 2 | ~141 |
| **Total** | **254** |

---

## 3. Metadata Linking

**Goal**: Map each paper to a DOI and BibTeX.

### IEEE S&P DOI pattern

| Field | Pattern | Example (2025) |
|---|---|---|
| DOI | `10.1109/SP{conf_num}.{year}.{id}` | `10.1109/SP61157.2025.00193` |
| URL | `https://doi.org/{doi}` | `https://doi.org/10.1109/SP61157.2025.00193` |
| DBLP key | `conf/sp/{AuthorKey}{year}` | `conf/sp/WangLLH0Z25` |
| DBLP bib | `https://dblp.org/rec/{key}.bib` | `https://dblp.org/rec/conf/sp/WangLLH0Z25.bib` |

Where `{conf_num}` is the IEEE conference number (e.g., `61157` for 2025).

### Current limitation

- IEEE Xplore proceedings are not yet published for S&P 2026.
- DBLP has not yet indexed S&P 2026.
- The DOI `{id}` (e.g., `00193`) cannot be derived from the accepted page content.

### Backfill strategy

Two possible approaches once proceedings are available:

**Option A: IEEE Xplore**
1. Locate S&P 2026 on IEEE Xplore (`https://ieeexplore.ieee.org/xpl/conhome/{conf_num}/proceeding`).
2. Scrape the paper list to get title → DOI mapping.
3. Download BibTeX via `https://doi.org/{doi}` (accept header `text/bibtex`).

**Option B: DBLP**
1. Wait for `https://dblp.org/db/conf/sp/sp2026.html` to be indexed.
2. Download the full-volume BibTeX: `https://dblp.org/db/conf/sp/sp2026.bib`.
3. Parse and match by title.

### Placeholder record format

```json
{
  "title": "Bridge: High-Order Taint Vulnerabilities Detection in Linux-based IoT Firmware",
  "author": "Jiaqian Peng1,2, Puzhuo Liu3, ..., 1: Institute of Information Engineering...",
  "url": "",
  "bib": "",
  "doi": ""
}
```

---

## 4. Batch Retrieval

Same concurrency and retry strategy as the ACL workflow. Key points:

- When backfilling from DBLP volume `.bib`: download a single file containing all papers, then parse.
- When backfilling from IEEE Xplore: one request per paper via DOI with `Accept: application/x-bibtex` header.
- Use `ThreadPoolExecutor(max_workers=20)` for individual DOIs.
- Batches of 50 papers per checkpoint.

---

## 5. Assembly & Validation

### JSONL format (initial)

```json
{
  "title": "Paper Title",
  "author": "Author1, Author2, ... (with affiliations)",
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
| All records have DOI | Filter where `doi == ""` |
| All records have BibTeX | Filter where `bib == ""` |
| Duplicate titles | `len(set(titles)) == len(titles)` |
| All parse as JSON | `json.loads(line)` in a loop |

---

## 6. Incremental Maintenance

### Two-phase strategy

**Phase 1 (before proceedings)**
- Scrape accepted page → save 254 titles + raw authors.
- Write placeholder jsonl with empty url/bib/doi.

**Phase 2 (after proceedings published)**
- Monitor IEEE Xplore or DBLP for S&P 2026 availability.
- Download BibTeX (via DBLP volume export or per-paper DOI).
- Match against Phase 1 records by title.
- Update jsonl with url, bib, doi in-place.

### Monitoring

```python
def check_dblp_ready():
    resp = urllib.request.urlopen(
        "https://dblp.org/db/conf/sp/sp2026.html", timeout=10
    )
    return resp.status == 200

def check_ieee_ready():
    resp = urllib.request.urlopen(
        "https://doi.org/10.1109/SP61157.2026.00001", timeout=10
    )
    return resp.status == 200
```

### Re-running

- Script detects existing records and only backfills empty `url`/`bib`/`doi` fields.
- Already-filled records are skipped.

---

## Ecosystem Reference

| Conference | DOI prefix | BibTeX source |
|---|---|---|
| IEEE S&P | `10.1109/SP{num}.{year}` | DBLP or IEEE Xplore DOI |
| IEEE S&P (older) | `10.1109/SP.{year}` | DBLP |
| IEEE CSF | `10.1109/CSF{num}.{year}` | DBLP or IEEE Xplore |
| USENIX Security | — | USENIX website |
| NDSS | — | NDSS proceedings page |

---

## Glossary

| Term | Definition |
|---|---|
| IEEE conference number | Numeric identifier for an IEEE conference, e.g. `61157` for S&P 2025 |
| Review cycle | S&P uses multi-cycle reviews; Cycle 1 and Cycle 2 papers are listed separately |
| DBLP volume bib | A single BibTeX file containing all papers in a proceedings volume |
| Backfill | Process of filling previously empty fields (url, bib, doi) once data becomes available |
