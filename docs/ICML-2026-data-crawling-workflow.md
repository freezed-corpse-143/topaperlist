# ICML 2026 Data Crawling Workflow

This document describes the pipeline for collecting ICML 2026 paper metadata and producing the standardized JSONL + TXT files.

---

## 1. Discovery

| Item | Value |
|---|---|
| Conference | 42nd International Conference on Machine Learning (ICML 2026) |
| Location | Vancouver, Canada |
| Conference dates | July 7–9, 2026 |
| Papers page | `https://icml.cc/virtual/2026/papers.html` |
| API endpoint | `https://icml.cc/static/virtual/data/icml-2026-orals-posters.json` |
| Format | Paginated JSON REST API (count + results array) |
| BibTeX | Embedded in JSON (auto-generated) |
| Total papers extracted | 6,796 (includes regular, oral, spotlight, and position papers) |

### Verdict

- Conference is happening this week (July 7–9, 2026).
- Full accepted papers list is live on the virtual site.
- Data sourced from the underlying JSON API (not HTML scraping).
- All records include URL and auto-generated BibTeX.

---

## 2. Source Parsing

### API structure

The JSON API returns:

```json
{
  "count": 6796,
  "next": null,
  "previous": null,
  "results": [
    {
      "id": 64393,
      "uid": "7f945d34...",
      "name": "Paper Title",
      "authors": [
        {"id": 229393, "fullname": "Author Name", "institution": "University"}
      ],
      "topic": "Deep Learning->Foundation Models",
      "keywords": [],
      "decision": "Accept (regular)",
      "session": "Poster Session 2",
      "event_type": "Poster",
      "room_name": "HALL A",
      ...
    }
  ]
}
```

### Extraction

The API is a single endpoint with all results; no pagination needed:

```javascript
const resp = await fetch('https://icml.cc/static/virtual/data/icml-2026-orals-posters.json');
const data = await resp.json();

const lines = data.results.map(p => ({
  title: p.name,
  author: p.authors.map(a => a.fullname).join('; '),
  url: `https://icml.cc/virtual/2026/poster/${p.id}`,
  bib: `@inproceedings{ICML2026_${p.id},\n  title = {${p.name}},\n  author = {${p.authors.map(a => a.fullname).join(' and ')}},\n  booktitle = {International Conference on Machine Learning},\n  year = {2026},\n  url = {https://icml.cc/virtual/2026/poster/${p.id}}\n}`,
}));
```

### Paper count verification

| Category | Count |
|---|---|
| JSON API total | 6,796 |
| TXT lines | 6,795 |
| JSONL lines | 6,795 |

Note: TXT/JSONL file line count is one less than API count because the last newline is not counted by `wc -l`.

---

## 3. Metadata Quality

- **Authors**: Full names with semicolon separators, matching existing ICML format.
- **URL**: Each paper has a unique virtual poster page URL.
- **BibTeX**: Auto-generated from API data; uses `ICML2026_{poster_id}` as citation key.
- **DOI**: Not available from the ICML API. Backfill from proceedings when published.

---

## 4. Assembly & Validation

### JSONL format

```json
{
  "title": "TVI-CoT: Text-Visual Interleaved Chain-of-Thought Reasoning for Multimodal Understanding",
  "author": "Lianyu Hu; Xiaoyu Ma; Zeqin Liao; Yang Liu",
  "url": "https://icml.cc/virtual/2026/poster/64393",
  "bib": "@inproceedings{ICML2026_64393,\n  title = {...},\n  author = {...},\n  booktitle = {International Conference on Machine Learning},\n  year = {2026},\n  url = {...}\n}",
  "doi": ""
}
```

### Validation checks

| Check | Result |
|---|---|
| Line count consistency | 6795 TXT == 6795 JSONL |
| All records have title | ✅ |
| All records have URL | ✅ |
| All records have BibTeX | ✅ |
| Duplicate titles | To be checked |

---

## 5. Backfill Strategy

- **DOI**: Not included in initial extract. ICML proceedings typically assign DOIs through PMLR (Proceedings of Machine Learning Research). Backfill via DOI matching when published.
- **BibTeX quality**: The auto-generated BibTeX is functional but lacks DOI, page numbers, and publisher fields. Replace with PMLR proceedings BibTeX when available.
- **Monitoring**: Check `https://proceedings.mlr.press/` for ICML 2026 volume.

---

## Ecosystem Reference

| Conference | Data source | BibTeX source |
|---|---|---|
| ICML | `icml.cc` API (`/static/virtual/data/`) | Auto-generated / PMLR proceedings |
| NeurIPS | `neurips.cc` API | Auto-generated / proceedings |
| ICLR | OpenReview API | OpenReview export |
