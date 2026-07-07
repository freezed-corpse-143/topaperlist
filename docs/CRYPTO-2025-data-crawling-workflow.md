# CRYPTO 2025 Data Crawling Workflow

This document describes the pipeline for collecting CRYPTO 2025 paper metadata and producing the standardized JSONL + TXT files.

---

## 1. Discovery

| Item | Value |
|---|---|
| Conference | 45th Annual International Cryptology Conference (CRYPTO 2025) |
| Location | Santa Barbara, CA, USA |
| Conference dates | August 17–21, 2025 |
| DBLP source | `https://dblp.org/db/conf/crypto/crypto2025-{1..6}.html` (6 parts) |
| Proceedings | Springer LNCS, 6 volumes (16000–16005) |
| Editors | Yael Tauman Kalai, Seny F. Kamara |
| Total papers extracted | 125 |

### Verdict

- Conference already held (August 2025).
- Proceedings published as 6 Springer LNCS volumes.
- DBLP has fully indexed all 6 parts.
- Data extracted from DBLP page text.

---

## 2. Source Structure

CRYPTO 2025 proceedings are divided into 6 Springer LNCS volumes:

| Part | LNCS Volume | Topics |
|---|---|---|
| Part I | 16000 | Mathematical Foundations, Isogeny-Based Crypto, Code-Based Crypto, Lattice Crypto |
| Part II | 16001 | Blockchain, Consensus, Quantum Cryptography |
| Part III | 16002 | Functional Encryption, Homomorphic Encryption, Anamorphic Encryption |
| Part IV | 16003 | MPC, Secret Sharing, Garbled Circuits |
| Part V | 16004 | Side-Channel Attacks, Symmetric Crypto, Block Ciphers |
| Part VI | 16005 | Signatures, Zero-Knowledge, Polynomial Commitments |

### DBLP page format

Each DBLP page lists papers in a clean text format:

```
Author1, Author2, ...:
Title.  Page-Page
```

Sections are divided by topic headers (e.g., "Mathematical Foundations and Isogeny-Based Cryptography").

---

## 3. Extraction

### Page text method

Navigate to each DBLP page and extract the rendered text:

```javascript
const text = await tab.evaluate(() => document.body.innerText);
const lines = text.split('\n').filter(l => l.trim());

for (let i = 0; i < lines.length; i++) {
  if (line.endsWith(':') && i + 1 < lines.length) {
    const authors = line.slice(0, -1).trim();
    const nextLine = lines[i + 1];
    const m = nextLine.match(/^(.*?)\s+\d+-\d+$/);
    if (m) {
      let title = m[1].trim();
      if (title.endsWith('.')) title = title.slice(0, -1);
      // save { title, authors }
    }
  }
}
```

### Page requirements

- All 6 DBLP pages are static HTML (no JavaScript rendering needed for content).
- The `read` tool works directly for individual pages, but the page text must be parsed cleanly.
- Skip: editorship entries, section headers, navigation/footer text.

### Paper counts per part

| Part | Papers |
|---|---|
| Part I | 22 |
| Part II | 21 |
| Part III | 20 |
| Part IV | 21 |
| Part V | 21 |
| Part VI | 20 |
| **Total** | **125** |

---

## 4. Metadata Linking

### DOI pattern

CRYPTO papers use Springer DOI format: `10.1007/978-3-032-{volume_id}.{paper_id}`

### Current limitation

- DOIs are not included in the initial extract.
- DBLP pages do not show DOIs directly; they link to Springer for each paper.
- BibTeX is available from Springer LNCS or DBLP individual paper pages.

### Backfill strategy

1. Wait for DBLP to expose the full-volume BibTeX file: `https://dblp.org/db/conf/crypto/crypto2025.bib` (one BibTeX per part or consolidated).
2. Or scrape individual paper DOIs from DBLP detail pages (each paper link includes a DOI).
3. Match by title against existing records.

---

## 5. Assembly & Validation

### JSONL format (initial)

```json
{
  "title": "PEGASIS: Practical Effective Class Group Action using 4-Dimensional Isogenies",
  "author": "Pierrick Dartois; Jonathan Komada Eriksen; Tako Boris Fouotsa; Arthur Herlédan Le Merdy; Riccardo Invernizzi; Damien Robert; Ryan Rueger; Frederik Vercauteren; Benjamin Wesolowski",
  "url": "",
  "bib": "",
  "doi": ""
}
```

### Validation checks

| Check | Result |
|---|---|
| Line count consistency | 124 TXT == 124 JSONL |
| All records have title | ✅ |
| All records have authors | ✅ |
| No duplicate titles | ✅ |

---

## 6. Backfill Strategy

- **DOIs**: Each paper has a Springer DOI. Extract from DBLP per-paper detail pages when DOI backfill is implemented.
- **BibTeX**: Springer LNCS provides BibTeX via the DOI landing pages (header `Accept: application/x-bibtex`).
- **Ready to check**: DBLP has already indexed all 6 parts, so BibTeX is available for per-paper download.

---

## Ecosystem Reference

| Conference | Data source | BibTeX source |
|---|---|---|
| CRYPTO | DBLP (multiple parts) | Springer LNCS DOI |
| EUROCRYPT | DBLP (multiple parts) | Springer LNCS DOI |
| ASIACRYPT | DBLP | Springer LNCS DOI |
| TCC | DBLP | Springer LNCS DOI |
