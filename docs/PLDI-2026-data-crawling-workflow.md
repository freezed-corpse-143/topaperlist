# PLDI 2026 Data Crawling Workflow

This document describes the pipeline for collecting PLDI 2026 paper metadata and producing the standardized JSONL + TXT files.

---

## 1. Discovery

| Item | Value |
|---|---|
| Conference | 47th ACM SIGPLAN Conference on Programming Language Design and Implementation (PLDI 2026) |
| Location | Boulder, Colorado, USA |
| Conference dates | June 15–19, 2026 |
| Track page | `https://pldi26.sigplan.org/track/pldi-2026-papers` |
| Format | Dynamic HTML via `conf.researchr.org` (JavaScript-rendered) |
| BibTeX | Auto-generated from DOI (ACM DL) |
| DBLP page | Not yet indexed (PLDI papers appear in PACMPL journal, not as standalone proceedings) |
| Total papers extracted | 106 |

### Verdict

- Conference already held (June 2026).
- Full accepted papers list accessible via the "Accepted Papers" tab.
- All papers have DOIs from ACM Digital Library.
- Data extracted from the conference website's Accepted Papers table.

---

## 2. Source Structure

### Page type

The PLDI 2026 site is built on `conf.researchr.org`. The track page loads a JavaScript-rendered schedule. An "Accepted Papers" sub-tab provides a simple table of all papers.

### Accepted Papers table format

The table has two columns: a bookmark icon and the paper entry:

```
Title
Author1, Author2, ...
DOI
```

Each row contains:
- Paper title (clickable link, `#` href)
- Author links (each links to `/profile/{author-id}`)
- DOI link (`https://doi.org/10.1145/{id}`)

### Extraction

Navigate to the track page, click the "Accepted Papers" tab, then extract from the table:

```javascript
// Navigate and switch to Accepted Papers tab
await tab.goto('https://pldi26.sigplan.org/track/pldi-2026-papers?', { waitUntil: 'networkidle0' });
await tab.click('aria/Peer Review Process'); // or use the proper ref
await new Promise(r => setTimeout(r, 2000));

// Extract from the table
const papers = await tab.evaluate(() => {
  const rows = document.querySelectorAll('table tbody tr');
  return rows.map(row => {
    const cells = row.querySelectorAll('td');
    if (cells.length < 2) return null;
    const titleLink = cells[1].querySelector('a');
    if (!titleLink) return null;
    const title = titleLink.textContent.trim();
    const authorLinks = cells[1].querySelectorAll('a');
    const authors = [];
    authorLinks.forEach(a => {
      if (a !== titleLink && a.href.includes('/profile/')) {
        authors.push(a.textContent.trim());
      }
    });
    const doiLink = cells[1].querySelector('a[href*="doi.org"]');
    const doi = doiLink ? doiLink.href : '';
    return { title, authors: authors.join('; '), doi };
  }).filter(Boolean);
});
```

### Paper count

| Category | Count |
|---|---|
| Research papers | 106 |
| Keynotes (not included) | 3 |

---

## 3. Metadata Quality

- **Authors**: Full names parsed from profile links, semicolon-separated.
- **DOIs**: All 106 papers have ACM DOIs (`10.1145/{id}`).
- **BibTeX**: Auto-generated from DOIs using standard ACM PLDI format.
- **URL**: Not included in initial extract (paper page URLs are hash-only `#`).

### Known issues

- DOIs are from the ACM Digital Library pre-publication. Some may resolve to "In Proceedings of..." while others may be pre-prints.
- Some entries include "[SIGPLAN OOPSLA'25]" or "[TOPLAS]" prefixes — these are joint or invited papers included in the PLDI program.

---

## 4. Assembly & Validation

### JSONL format (initial)

```json
{
  "title": "Abstract Interpretation with Confidence",
  "author": "Yuanfeng Shi; Ziyue Jin; Xin Zhang",
  "url": "",
  "bib": "@inproceedings{PLDI2026_10_1145_3808351,\n  title = {Abstract Interpretation with Confidence},\n  author = {Yuanfeng Shi and Ziyue Jin and Xin Zhang},\n  booktitle = {Proceedings of the 47th ACM SIGPLAN Conference on Programming Language Design and Implementation},\n  year = {2026},\n  doi = {10.1145/3808351}\n}",
  "doi": "10.1145/3808351"
}
```

### Validation checks

| Check | Result |
|---|---|
| Line count consistency | 106 TXT == 106 JSONL |
| All records have title | ✅ |
| All records have DOI | ✅ |
| Duplicate titles | ✅ (no duplicates) |

---

## 5. Backfill Strategy

- **BibTeX quality**: Auto-generated BibTeX is a reasonable starting point. Replace with official ACM BibTeX (via DOI with `Accept: application/x-bibtex`) for canonical citation keys.
- **DBLP monitoring**: Once indexed at `https://dblp.org/db/conf/pldi/pldi2026.html`, the full proceedings BibTeX can be used as a single authoritative source.
- **Conference URLs**: The paper landing pages on `pldi26.sigplan.org` are hash-only. ACM DL links via DOI are the canonical URLs.

---

## Ecosystem Reference

| Conference | Data source | BibTeX source |
|---|---|---|
| PLDI | conf.researchr.org Accepted Papers | ACM DL DOI |
| POPL | conf.researchr.org | ACM DL |
| OOPSLA | conf.researchr.org | ACM DL |
| ICFP | conf.researchr.org | ACM DL |
