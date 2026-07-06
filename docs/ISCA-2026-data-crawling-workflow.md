# ISCA 2026 Data Crawling Workflow

This document describes the pipeline for collecting ISCA 2026 paper metadata and producing the standardized JSONL + TXT files.

---

## 1. Discovery

| Item | Value |
|---|---|
| Conference | 53rd IEEE/ACM International Symposium on Computer Architecture (ISCA 2026) |
| Location | Raleigh, NC, USA |
| Conference dates | June 27 – July 1, 2026 |
| Program page | `https://iscaconf.org/isca2026/program/` |
| Proceedings | Published via ACM Digital Library (`10.1145/{number}`) |
| DBLP page | Not yet indexed |
| Total papers extracted | 154 |

### Verdict

- Conference already concluded. Program page lists all papers with session schedule.
- No dedicated "accepted papers" page — data is extracted from the program/schedule page.
- BibTeX pending DBLP indexing or individual DOI lookups via ACM DL.

---

## 2. Source Parsing

### Page structure

The ISCA program page is a single HTML page with expandable session blocks. Papers are listed under session headings with time slots:

```html
10:15 – 10:35
MLX: Multi-Layer Execution for Structured LLM Workload Acceleration on Spatial Architectures
Haibin Wu (Institute of Computing Technology, CAS), Wenming Li (...), ...
```

The markdown extraction (via browser `tab.extract('markdown')`) renders each paper as:

```
10:15 – 10:35

TITLE

Author1 (Affil), Author2 (Affil), ...

* * *
```

### Extraction method

Since the program page mixes paper entries with session metadata (time slots, session chairs, breaks), extraction requires context-based filtering:

```javascript
const md = await tab.extract('markdown');
const lines = md.split('\n');

for (let i = 0; i < lines.length; i++) {
  const line = lines[i].trim();
  if (!line || line.length < 15 || line.length > 200) continue;
  
  // Skip non-paper lines (time stamps, session headers, logistics)
  if (line.match(/^\d+:\d+/) || /* ... skip patterns ... */) continue;
  
  // Check if next-next line contains author affiliations
  const authorLine = (i + 2 < lines.length) ? lines[i + 2].trim() : '';
  if (authorLine && authorLine.match(/(University|Institute|School of|NVIDIA|...)/)) {
    // This is a paper entry
    papers.push({ title: line, authors: authorLine });
  }
}
```

### Known issues

- False positives from non-paper lines that happen to be followed by an affiliation-containing line (e.g., `Location: Ballroom A` followed by session chair info).
- Must filter out poster session entries, workshop entries, and Kizashi talk listings if only full papers are desired.
- Some paper titles appear multiple times in different sessions (e.g., best paper candidate sessions plus regular sessions) — deduplication is necessary.

---

## 3. Metadata Linking

### DOI pattern

ISCA papers use ACM DOI: `10.1145/{conference_id}.{paper_id}` (e.g., `10.1145/3695053.3731101`).

### Backfill approach

| Source | Method |
|---|---|
| ACM Digital Library | `https://dl.acm.org/doi/{doi}` with BibTeX export |
| DBLP | Wait for `https://dblp.org/db/conf/isca/isca2026.html` |
| DOI construction | Not possible without knowing the ACM conference ID |

### Challenges

- The program page does NOT include DOIs or paper IDs.
- Matching against ACM DL requires either DOI lookup by title or DBLP indexing.
- ACM conference ID for ISCA 2026 is unknown (2025 used `3695053`).

---

## 4. Assembly & Validation

### JSONL format (initial)

```json
{
  "title": "MLX: Multi-Layer Execution for Structured LLM Workload Acceleration on Spatial Architectures",
  "author": "Haibin Wu (Institute of Computing Technology, CAS), Wenming Li (...), ...",
  "url": "",
  "bib": "",
  "doi": ""
}
```

### Deduplication

The program page may list the same paper in multiple sessions (e.g., "Best Paper Candidate Session" + regular session). Deduplicate by title before saving.

---

## 5. Backfill Strategy

- Monitor DBLP for ISCA 2026 indexing.
- Once indexed, download from `https://dblp.org/rec/conf/isca/{key}.bib`.
- If DBLP is not available, construct ACM DL URLs by searching each paper title on `https://dl.acm.org/`.

---

## Ecosystem Reference

| Conference | Identifier | BibTeX source |
|---|---|---|
| ISCA | `10.1145/{conf_id}.{paper_id}` | ACM DL or DBLP |
| MICRO | `10.1109/MICRO{year}` | IEEE Xplore or DBLP |
| HPCA | `10.1109/HPCA{year}` | IEEE Xplore or DBLP |
| ASPLOS | `10.1145/{conf_id}.{paper_id}` | ACM DL or DBLP |
