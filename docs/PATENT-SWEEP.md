# Patent sweep (ROADMAP row 6): partial, claims NOT yet read

Status: **incomplete**. Search hits were found, but no claim was read verbatim, because every fetch of the Google Patents
pages returned HTTP 503 (two attempts each, 2026-11-22). Nothing here is a clearance, and nothing may be called "outside
as read" until the claims below are read.

## Ideas searched (what SortAnt implements)
RRF, Borda, CombSUM/MNZ/MAX/MIN/MED/ANZ (Fox and Shaw), ISR, DBSF, standardised score fusion, RBC, Condorcet, Copeland,
median rank. Fusion is old, published prior art; the sweep asks only whether an in-force claim reads on shipping them.

## Queries and results (Google Patents domain, web search snippets only)
| Query | Result | Control |
|---|---|---|
| `"reciprocal rank fusion"` claim combining ranked lists | US8838606B1 and US8713023B1 (active-learning classification; a dependent claim recites combining relevance rankings by reciprocal rank fusion, per the snippet); US8078606B2 (combining ranked lists from several search engines); US5864845A (multi-engine query clustering fusion) | the query returned patents that do contain the phrase, so it can find them |
| `"distribution-based score fusion"` / `"rank-biased centroids"` / `"inverse square rank"` | no hit using these phrases in claims; nearest is US10698908B2 (fuses relevancy scores by statistical distribution type) | the phrases are method names from papers, so zero patent hits proves little; not run against Espacenet or USPTO full text |

## To do before row 6 can close
1. Read the independent claims, verbatim, of US8838606B1, US8713023B1, US8078606B2, US5864845A and US10698908B2, with
   assignee, dates and legal status (try USPTO or Espacenet; Google Patents was down).
2. Run the same queries on a second source, each with a positive control.
3. Query the remaining methods by their classic names (CombMNZ, Borda count rank aggregation, Condorcet fusion, Copeland).
4. Record per patent: claim text, our feature, "outside as read" or "reads on", and the reason. Never "clear".
