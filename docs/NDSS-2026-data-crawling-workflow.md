# NDSS 2026 Data Crawling Workflow

This document describes the pipeline for collecting NDSS Symposium 2026 paper metadata and producing the standardized JSONL + TXT files.

---

## 1. Discovery

| Item | Value |
|---|---|
| Conference | 33rd Annual Network and Distributed System Security Symposium (NDSS 2026) |
| Location | San Diego, CA, USA |
| Conference dates | February 23–27, 2026 |
| Accepted papers page | `https://www.ndss-symposium.org/ndss2026/accepted-papers/` |
| Total accepted papers | 265 (Summer: 113, Fall: 152) |
| DBLP page | `https://dblp.org/db/conf/ndss/ndss2026.html` — **indexed** |
| DBLP bib export | `https://dblp.org/db/conf/ndss/ndss2026.bib` — **not available (404)** |

### Verdict

- **Partial**: 251 titles extracted. BibTeX stored in DBLP but no direct volume bib export.
- Per-paper DBLP rec keys exist (e.g., `conf/ndss/PanLLTFXCR26`), allowing individual bib download at `https://dblp.org/rec/conf/ndss/{key}.bib`.

---

## 2. Source Parsing

### Page structure

WordPress site. Papers rendered as `<h2><a href="...">Title</a></h2>` with author text in paragraphs below.

```
<h2>
  <a href="https://www.ndss-symposium.org/ndss-paper/{slug}/">
    Paper Title
  </a>
</h2>
Author1 (Affiliation1), Author2 (Affiliation2)...
```

### Extraction method

The page has anti-scraping protections (HTTP 403 from Python `urllib`). Use headless browser:

```javascript
// Using accessibility snapshot
const obs = await tab.observe({includeAll: true});
const titles = obs.elements
  .filter(e => e.role === 'heading' && e.name.length > 30
    && !e.name.includes('Privacy')
    && !e.name.includes('Accepted Paper'))
  .map(e => e.name);
```

### Page sections

| Section | Count |
|---|---|
| Summer Cycle 2026 | 113 |
| Fall Cycle 2026 | 152 |
| **Extracted** | **251** |

The extracted count (251) differs from the stated total (265) due to lazy-loaded content and heading filtering.

---

## 3. Metadata Linking

### DBLP key extraction

DBLP has indexed NDSS 2026. Individual paper rec links are embedded in the DBLP HTML page:

```
https://dblp.org/rec/conf/ndss/PanLLTFXCR26.html
```

The bib key is the path component: `conf/ndss/PanLLTFXCR26`.

### BibTeX download

Once keys are extracted, download:

```
https://dblp.org/rec/conf/ndss/{key}.bib
```

### Challenges

- DBLP page is JavaScript-rendered; keys not in raw HTML.
- No DBLP volume bib export (`ndss2026.bib` returns 404).
- Each paper requires an individual HTTP request for its bib.
- NDSS paper pages do not provide BibTeX directly.

### Backfill approach

1. Load DBLP page in headless browser.
2. Extract all rec links matching `rec/conf/ndss/{key}`.
3. Deduplicate keys (each appears as both `.html` and `.html?view=bibtex`).
4. Download bib for each key concurrently.
5. Match against extracted titles by normalized title comparison.

---

## 4. Assembly & Validation

### JSONL format (initial)

```json
{
  "title": "Paper Title",
  "author": "",
  "url": "",
  "bib": "",
  "doi": ""
}
```

### TXT format

One paper title per line, no other fields.

### Post-backfill fields

```json
{
  "title": "Paper Title",
  "author": "Author1; Author2; ...",
  "url": "https://dblp.org/rec/conf/ndss/{key}.html",
  "bib": "@inproceedings{DBLP:...}",
  "doi": ""
}
```

NDSS papers may not have DOIs; the bib URL serves as the canonical identifier.

---

## 5. Incremental Maintenance

### Phase 1 (done)
- Scrape accepted page → save 251 titles.
- Write placeholder jsonl with empty url/bib/doi.

### Phase 2 (pending)
- Open DBLP page to extract rec keys.
- Download individual bibs.
- Clean author strings (extract names from "Name (Affiliation)" format).
- Match and update jsonl records.

### Monitoring

```python
# Check if a specific paper bib is available
resp = urllib.request.urlopen(
    "https://dblp.org/rec/conf/ndss/PanLLTFXCR26.bib", timeout=10
)
```

---

## Ecosystem Reference

| Conference | BibTeX source | Identifier pattern |
|---|---|---|
| NDSS | DBLP individual rec | `conf/ndss/{AuthorKey}{year}` |
| IEEE S&P | DBLP or IEEE Xplore | `conf/sp/{key}` |
| USENIX Security | USENIX website | — |
| ACM CCS | ACM DL | DOI: `10.1145/{number}` |

---

## Glossary

| Term | Definition |
|---|---|
| DBLP rec key | Per-paper identifier in DBLP, e.g. `conf/ndss/PanLLTFXCR26` |
| Summer/Fall cycle | NDSS has two review cycles; paper lists are shown separately |
| Anti-scraping | Server blocks non-browser HTTP clients (HTTP 403) |
