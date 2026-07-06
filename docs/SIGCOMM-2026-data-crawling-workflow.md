# SIGCOMM 2026 Data Crawling Workflow

## Source
- URL: `https://conferences.sigcomm.org/sigcomm/2026/accepted/`
- Format: Static HTML page listing papers with authors and titles
- Papers: 109 (full papers, 1 header row excluded)
- BibTeX: Pending ACM DL indexing or DBLP

## Extraction
Authors and titles are concatenated on the same line: `Authors**TITLE**`. Extract by splitting on `**`.

## Known Issues
- First entry may have "Title / Authors" as author field (page header row).
- No DOIs on the page; backfill via ACM DL.
