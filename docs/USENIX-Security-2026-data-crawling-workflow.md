# USENIX Security 2026 Data Crawling Workflow

This document describes the pipeline for collecting USENIX Security '26 paper metadata and producing the standardized JSONL + TXT files.

---

## 1. Discovery

| Item | Value |
|---|---|
| Conference | 35th USENIX Security Symposium (USENIX Security '26) |
| Location | Baltimore, MD, USA |
| Conference dates | August 12–14, 2026 |
| Cycle 1 accepted papers | `https://www.usenix.org/conference/usenixsecurity26/cycle1-accepted-papers` |
| Cycle 2 accepted papers | `https://www.usenix.org/conference/usenixsecurity26/cycle2-accepted-papers` — **404 (not yet published)** |
| DBLP page | `https://dblp.org/db/conf/uss/uss2026.html` — **404 (not yet indexed)** |
| Total (Cycle 1 only) | 165 papers |

### Verdict

- **Partial**: 165 Cycle 1 titles + authors extracted. Cycle 2 not yet published.
- BibTeX pending DBLP indexing or USENIX proceedings publication.

---

## 2. Source Parsing

### Page structure

USENIX uses Drupal-based conference pages. Papers are rendered as:

```html
<h2>
  <a href="/conference/usenixsecurity26/presentation/{slug}">
    Paper Title
  </a>
</h2>

<p>Author1, <em>Affiliation1</em>; Author2, <em>Affiliation2</em></p>

<p>Available Media ...</p>
```

### Extraction method

Use `tab.extract('markdown')` which renders the page as:

```
## [Paper Title](/conference/usenixsecurity26/presentation/{slug})

Author1, _Affiliation1_; Author2, _Affiliation2_

Available Media
```

Parse with:

```javascript
// Match heading-2 links
const m = line.match(/^## \[(.+?)\]\((.+?)\)\s*$/);
// Author line is 2 lines after the title
const authorLine = lines[i + 2];
// Remove italic affiliations
const authors = authorLine.replace(/_[^_]+_/g, '').trim();
```

### Author cleaning

Raw author string includes affiliations in italics:

> `Jiaqian Peng, _Institute of Information Engineering, CAS_; Puzhuo Liu, _Ant Group; Tsinghua University_`

To extract only names, strip italicized sections:

```python
import re
clean = re.sub(r'_[^_]+_', '', author_string)
```

---

## 3. Metadata Linking

### USENIX Security ecosystem

| Resource | Pattern | Status |
|---|---|---|
| Paper page | `https://www.usenix.org/conference/usenixsecurity26/presentation/{slug}` | Online (Cycle 1) |
| DBLP key | `conf/uss/{AuthorKey}{year}` (e.g., `conf/uss/PengL0YLSZ26`) | Not yet indexed |
| DBLP bib | `https://dblp.org/rec/conf/uss/{key}.bib` | Pending DBLP |
| Proceedings | `https://www.usenix.org/conference/usenixsecurity26/technical-sessions` | Not yet published |

### Backfill approach

Once DBLP indexes USENIX Security 2026:

1. Parse DBLP HTML page for `rec/conf/uss/` links.
2. Extract unique DBLP keys.
3. Download individual bib files.
4. Match by normalized title.

---

## 4. Assembly & Validation

### JSONL format (initial)

```json
{
  "title": "Paper Title",
  "author": "Author1; Author2; ...",
  "url": "https://www.usenix.org/conference/usenixsecurity26/presentation/{slug}",
  "bib": "",
  "doi": ""
}
```

### TXT format

One paper title per line.

### Post-backfill fields

```json
{
  "title": "Paper Title",
  "author": "Author1; Author2; ...",
  "url": "https://dblp.org/rec/conf/uss/{key}.html",
  "bib": "@inproceedings{DBLP:...}",
  "doi": ""
}
```

---

## 5. Incremental Maintenance

### Phase 1 (done)
- Scrape Cycle 1 accepted page → save 165 titles + authors.
- Write placeholder jsonl with empty bib/doi.

### Phase 2 (pending)
- Monitor Cycle 2 page for publication.
- Monitor DBLP for USENIX Security 2026 indexing.
- Download bibs and update records.

### Phase 3 (pending)
- When Cycle 2 is published, scrape and append to existing records.
- Total expected: ~330 papers (165 per cycle × 2 cycles).

### Monitoring

```python
def check_cycle2():
    resp = urllib.request.urlopen(
        "https://www.usenix.org/conference/usenixsecurity26/cycle2-accepted-papers",
        timeout=10
    )
    return resp.status == 200

def check_dblp():
    resp = urllib.request.urlopen(
        "https://dblp.org/db/conf/uss/uss2026.html", timeout=10
    )
    return resp.status == 200
```

---

## Ecosystem Reference

| Conference | BibTeX source | Identifier |
|---|---|---|
| USENIX Security | DBLP (`conf/uss/{key}`) | DBLP rec key |
| USENIX ATC | DBLP (`conf/atc/{key}`) | DBLP rec key |
| OSDI | DBLP (`conf/osdi/{key}`) | DBLP rec key |
| NSDI | DBLP (`conf/nsdi/{key}`) | DBLP rec key |

---

## Glossary

| Term | Definition |
|---|---|
| Cycle | USENIX Security uses multi-cycle submissions; each cycle has its own accepted papers page |
| DBLP key | Per-paper identifier in DBLP, e.g. `conf/uss/PengL0YLSZ26` |
| Slug | URL-friendly paper identifier, e.g. `peng-jiaqian` |
