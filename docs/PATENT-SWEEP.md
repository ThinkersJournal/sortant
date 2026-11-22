# Patent sweep (ROADMAP row 6): partial, two of five hits read

Status: **incomplete**. Five patents were found; the claims of two are read below. The Google Patents pages first returned
HTTP 503, then served pages whose claims a fetch tool would not reproduce, so the claims came from the PDFs. Nothing here is
a clearance.

## Ideas searched (what SortAnt implements)
RRF, Borda, CombSUM/MNZ/MAX/MIN/MED/ANZ (Fox and Shaw), ISR, DBSF, standardised score fusion, RBC, Condorcet, Copeland,
median rank. Fusion is old, published prior art; the sweep asks only whether an in-force claim reads on shipping them.

## Queries and results (Google Patents domain, web search snippets only)
| Query | Result | Control |
|---|---|---|
| `"reciprocal rank fusion"` claim combining ranked lists | US8838606B1 and US8713023B1 (active-learning classification; a dependent claim recites combining relevance rankings by reciprocal rank fusion, per the snippet); US8078606B2 (combining ranked lists from several search engines); US5864845A (multi-engine query clustering fusion) | the query returned patents that do contain the phrase, so it can find them |
| `"distribution-based score fusion"` / `"rank-biased centroids"` / `"inverse square rank"` | no hit using these phrases in claims; nearest is US10698908B2 (fuses relevancy scores by statistical distribution type) | the phrases are method names from papers, so zero patent hits proves little; not run against Espacenet or USPTO full text |

## Claims read (2026-11-22)

Source for the two patents below: the Google Patents PDF (`patentimages.storage.googleapis.com`), text extracted with
`pdftotext`. The PDF is two-column, so the extracted text interleaves columns; I reassembled each claim by reading its
own words in order. This is not the USPTO's copy. Bibliographic data is from the Google Patents page (its status line is
Google's assumption, not a legal conclusion).

| Patent | Status | Independent claim, as read | RRF appears | Our feature | Reading |
|---|---|---|---|---|---|
| US8838606B1 (Cormack, Grossman; now Adaptive Classification Technologies LLC; filed 2013-06-18) | active, anticipated expiry 2033-03-15 | claim 1: "An active learning system for processing documents in connection with conducting e-discovery in a legal proceeding" with a processor that generates document information profiles, presents a document to "a human reviewer", receives "a user coding decision", calculates and incrementally updates "a classifier", computes scores, estimates recall "by fitting scores ... to a standard distribution", and classifies documents "as relevant or non-relevant to the legal proceeding" | only claim 6, depending on 5 and 1: "the output of the relevance rankings techniques are combined using reciprocal rank fusion" | `RankMethod::Rrf` | **outside as read**: every claim needs claim 1's e-discovery review loop, a human reviewer, a classifier and recall estimation; sortant has none, and RRF is only a dependent-claim detail |
| US8713023B1 (same inventors; filed 2013-06-18) | active, anticipated expiry 2033-03-15 | claim 1: "A system for classifying documents in a document collection as relevant or non-relevant in connection with conducting e-discovery in a legal proceeding" with a memory, a display, a physical input interface and a processor that selects a document for "a human reviewer", receives "user coding decisions", updates a classifier "using an incremental learning technique", fits scores "to a standard distribution curve" and estimates the number of relevant documents | only claim 5, depending on 4, 3 and 1: "an output of the relevance rankings techniques are combined using reciprocal rank fusion" | `RankMethod::Rrf` | **outside as read**, same reason |

These two readings are about shipping the library. A downstream product that builds the claimed e-discovery system and
uses `Rrf` inside it is a different question and is not assessed here.

## Still unread

US8078606B2 (AT&T, filed 2009, Google says expired 2023-06-27), US5864845A (Siemens, filed 1996, Google says expired
2016-06-28) and US10698908B2 (IBM, filed 2016, Google says active to 2037-10-18). Only a model's paraphrase of their
claims exists, and it is not evidence. For US10698908B2 the paraphrase says dependent claims 4, 11 and 18 list CombSUM,
CombMNZ and LambdaMerge as fusion models, which makes it the one to read first because it is active and names methods
sortant implements. Read its PDF the same way.

## To do before row 6 can close
1. Read the independent and fusion-naming dependent claims of US10698908B2 first, then US8078606B2 and US5864845A, from
   the PDFs, and record them in the table above.
2. Run the same queries on a second source (USPTO full text or Espacenet), each with a positive control.
3. Query the remaining methods by their classic names (CombMNZ, Borda count rank aggregation, Condorcet fusion, Copeland).
4. Never write "clear"; "outside as read" only, per patent, with the reason.
