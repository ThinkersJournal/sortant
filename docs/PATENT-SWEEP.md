# Patent sweep (ROADMAP row 6): partial, nine hits read

Status: **incomplete**. **Single source: Google Patents only** (board 511, second source deferred by CireSnave's PM; re-check the quoted limbs if one is chosen). The independent claims of nine hits are read below. The Google Patents pages first returned
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
| US8838606B1 (Cormack, Grossman; now Adaptive Classification Technologies LLC; filed 2013-06-18) | active, expiry per Google 2034-02-26, read 2026-11-26 (this table said 2033-03-15 on the first read) | claim 1: "An active learning system for processing documents in connection with conducting e-discovery in a legal proceeding" with a processor that generates document information profiles, presents a document to "a human reviewer", receives "a user coding decision", calculates and incrementally updates "a classifier", computes scores, estimates recall "by fitting scores ... to a standard distribution", and classifies documents "as relevant or non-relevant to the legal proceeding" | only claim 6, depending on 5 and 1: "the output of the relevance rankings techniques are combined using reciprocal rank fusion" | `RankMethod::Rrf` | **outside as read (single source: Google Patents)**: every claim needs claim 1's e-discovery review loop, a human reviewer, a classifier and recall estimation; sortant has none, and RRF is only a dependent-claim detail |
| US8713023B1 (same inventors; filed 2013-06-18) | active, expiry per Google 2034-02-26, read 2026-11-26 (first read: 2033-03-15) | claim 1: "A system for classifying documents in a document collection as relevant or non-relevant in connection with conducting e-discovery in a legal proceeding" with a memory, a display, a physical input interface and a processor that selects a document for "a human reviewer", receives "user coding decisions", updates a classifier "using an incremental learning technique", fits scores "to a standard distribution curve" and estimates the number of relevant documents | only claim 5, depending on 4, 3 and 1: "an output of the relevance rankings techniques are combined using reciprocal rank fusion" | `RankMethod::Rrf` | **outside as read (single source: Google Patents)**, same reason |

| US10698908B2 (IBM; Cohen, Roitman; filed 2016-07-12) | active, expiry per Google 2037-10-18, read 2026-11-26 (unchanged) | claim 1 (claims 8 and 15 parallel it): a method of "receiving, from a computerized search engine, a group of relevancy score sets in response to a multi-field search query", each set denoting "a relevancy of one of the multiple document fields to one of the multiple search fields" and "determined by the computerized search engine based on a different information retrieval model"; computing "at least one respective statistical value" per set; and combining by "(a) filtering, out of the group, one or more of the relevancy score sets whose respective statistical value indicates at least one of: zero variance of relevancy scores, and random relevancy scoring, (b) segmenting the group into a plurality of subgroups according to the respective statistical values ..., (c) fusing the relevancy score sets in each of the subgroups ..., and (d) combining the fused relevancy score sets", then "sending the combined relevancy score set to the computerized search engine" | claims 7 and 9-10 name normalising to a normal distribution and per-subgroup fusion techniques; I did not see the text of claims 4, 11 and 18 (the ones a paraphrase said list CombSUM/CombMNZ) in this extract | `Dbsf`, `Standardized`, `CombSum`, `CombMnz` | **outside as read (single source: Google Patents)**: claim 1 needs a multi-field query result from a search engine, filtering of score sets by variance, segmentation into subgroups by statistic, per-subgroup fusion and a send-back step; `fuse` does none of the filtering, segmentation or two-level fusion, and gives a constant list 0.5 instead of removing it |
| US8078606B2 (AT&T, later Google; filed 2009, continuation) | expired per Google (expiry 2023-06-27, read 2026-11-26, unchanged), a Google assumption | claim 1: "Submitting a query to multiple search engines", "receiving ... a ranked list of search results from each search engine", "determining a first weighting value of a first search engine ... specialized in a first category and wherein the first weighting value indicates a degree of relevance of the first category to a category of the query", the same for a second engine, and "determining a final relevance value for each search result" by applying the engine's weighting value and the result's relevance value (claims 10 and 15 parallel it) | none seen | `Weighted`, per-list weights | **outside as read (single source: Google Patents)**: the weights are derived from each engine's specialisation category against the query's category; sortant takes caller-supplied weights and has no categories. Expired in any case, per Google |
| US5864845A (Siemens; filed 1996) | expired per Google (expiry 2016-06-28, read 2026-11-26, unchanged), a Google assumption | claim 1: "A method implemented on a computer for facilitating World Wide Web Searches ... by combining search result documents, as provided by separate search engines ... into one single integrated list" with the steps of "training said computer for each search engine by clustering training queries and building cluster centroids" and assigning weights to each cluster (the extract is column-interleaved and I read only the opening steps) | none seen | `Weighted`, `CombSum` | **outside as read (single source: Google Patents)**: the claim is built on training on query clusters; sortant has no training or query clustering. Expired in any case, per Google |
| US11163780B2 (IBM; "Utilizing passages in fusion-based document retrieval") | "Expired - Fee Related" per Google | claim 1: receiving a query; retrieving documents "using a plurality of retrieval methods, each ... generating a ranked list ... and a score for each document"; "fusing ... the generated ranked list ... by: re-scoring ... the plurality of documents according to its passage scores ... using at least one of CombMaxPsg, or PsgPFF" (as extracted: "Comb MaxPsg") and "associating ... a given document and its highest scored passage using information indicating a relevance of the passage induced from the plurality of ranked lists" | its description and dependent claims name CombSUM, CombMNZ, Borda and RRF as the base fusion (from a search snippet, not read in the claims) | all fusion methods | **outside as read (single source: Google Patents)**: claim 1 needs passage-level re-scoring and passage association; sortant sees only document ids and scores. Expired per Google (fee-related), which I did not verify against the USPTO |
| US7188106B2 (IBM; aggregating ranking results from various sources) | status not read | claim 1: "constructing an initial aggregated ranking from a plurality of source rankings"; "computing a distance measure comparing a plurality of source rankings output by at least one search engine that has incorrectly estimated the relevance of at least one web page"; "altering elements in an altered aggregated ranking to minimize said distance measure", an element being inserted only if a majority of the source rankings prefers it to the lowest-ranked element and to all successors of that element; "outputting the altered aggregated ranking" | not applicable | `Condorcet`, `Copeland` | **outside as read (single source: Google Patents)**: the claim is a local-Kemenization procedure that starts from an initial aggregate and rewrites it to minimise a distance measure under a majority insertion rule. `Condorcet` and `Copeland` count pairwise preferences once and sort; neither builds or alters an aggregated ranking against a distance measure. This is the closest hit to our pairwise methods, so it deserves a second reader |
These readings are about shipping the library. A downstream product that builds the claimed e-discovery system and
uses `Rrf` inside it is a different question and is not assessed here.

## Second round, read 2026-11-26

Source: the Google Patents pages (`/patent/<number>/en`), claims section, which served normally this time. Status lines are
Google's, not a legal conclusion. Queries that found them: see the second-round queries below.

| Patent | Status | Independent claims, as read | RRF appears | Our feature | Reading |
|---|---|---|---|---|---|
| US12259913B1 (Inventus Holdings per Google; "Caching LLM responses using hybrid retrieval and RRF") | active per Google | claims 1, 8, 11: a cache-retrieval method or system that vectorises a question into a vector of "greater than or equal to 1024 dimensions" with a text embedding algorithm, runs a hybrid search over a question portion of a cache, filters by metadata, produces a semantic layer set and a lexical layer set, and "applying a reciprocal rank fusion algorithm to compute a combined ranking set" to "provide an identified answer"; claim 8 adds returning the answer only when the combined ranking is above a settable value | every independent claim | `Rrf` | **outside as read (single source: Google Patents)**: each claim needs an LLM-answer cache, question embedding of 1024+ dimensions and semantic and lexical layer sets; sortant takes ranked lists or scores and embeds, caches and filters nothing. RRF is one step inside the claimed pipeline. A downstream product that builds that cache is not assessed here |
| US2025/0094398A1 (Oracle; "Unified RDBMS framework for hybrid vector search") | pending application per Google | claims 1, 16, 17: building a vector table and a text table from documents (plaintext conversion, chunking, embedding, tokenising, document ids), and in claim 16 a type-specific conversion plus vector and text indexes; dependent claims 14-15 combine two result sets by union or intersection of document ids | none: the words "reciprocal" and "fusion" do not occur in the claims | none | **outside as read (single source: Google Patents)**: the independent claims are about database storage and indexing, not score or rank fusion. It is an application, so claims may change; recheck if it grants |

## Second source attempt, 2026-11-26: blocked

Tried without any key: freepatentsonline.com search (HTTP 403 "forbidden by administrative rules"), Espacenet REST search 
(HTTP 403; its OPS API needs a registered key, which this lane does not hold), Justia patent page (HTTP 403). USPTO Patent Public 
Search (`ppubs.uspto.gov`) loads but is a script-driven session app with no documented keyless query endpoint, so it was not driven. 
No second-source query has run, so the "second source" item below stays open and needs either a person at a browser on 
Espacenet/USPTO or a key CireSnave approves.

## Classic-name queries, 2026-11-26 (single source: Google Patents)

Queries (web search restricted to Google Patents pages): `"CombMNZ" OR "Borda count"` rank aggregation merging ranked lists; `"Condorcet" OR "Copeland"` fusion of ranked lists search engine results. Control: both returned US7188106, which contains "Condorcet" and "Borda" in its description. No hit names Copeland or CombMNZ in its claims as far as the claims below show (claims read from the Google Patents pages, `en` view; legal status is Google's line, not a legal conclusion). Nothing here is a clearance.

| Patent | Status per Google | Independent claim, as read | Our feature | Reading (single source: Google Patents) |
|---|---|---|---|---|
| US6728704B2 (merging result lists from multiple search engines; claims 1, 7, 12, 18) | Expired - Lifetime | claim 1: transmit a query to a set of search engines, receive a result list from each, "selecting a subset of entries from each result list", assign scoring values by a scoring function, "assigning to each subset a representative value", produce a merged list "based on the representative value assigned to each result list" | `Weighted`, `CombSum`, `Borda` | **outside as read (single source: Google Patents)**: needs query transmission, subset selection per list and a per-list representative value derived from its entries' scores; `fuse` takes lists as given and uses caller-supplied weights. Expired per Google |
| US7716202B2 (weighted relevance value per result; 18 claims) | Expired - Fee Related | claim 1: submit a query to several engines; for a list "without actual relevance values", estimate a value for the most relevant result and for "a less relevant search result" and derive the others from those two; determine a weighting value per engine; compute a weighted relevance value per result; combine into a single list, sort, send for display | `Borda` and `Weighted` (rank to points, per-list weights) | **needs more (single source: Google Patents)**: closest hit to Borda with weights, because rank-to-score by interpolation between two anchors can resemble Borda points; the claim also needs query submission, the estimated-value steps and sending to a display, which `fuse` does not do. Expired per Google (fee-related), unverified on the USPTO; second reader wanted |
| EP1631901A2 (blending search engine results from disparate sources) | Withdrawn | claim 1: receive from each source a result list with position, document address and relevance score; "applying a determined transformation to the relevance scores" of at least one list; combine lists into a blended list ordered "at least in part on the transformed scores" (claims 2-3 derive the transformation from click-rate statistics) | `MinMax`/`Dbsf`/`Standardized` then `CombSum` | **needs more (single source: Google Patents)**: claim 1 as extracted is broad enough that score normalisation followed by combination could be argued to fall within it; the application is withdrawn per Google, so no grant is expected, but the European register was not read |
| US7962500B2 (image retrieval by aggregating search results on visual annotations) | Expired - Fee Related | claim 1: keyword request, annotated images with bounded regions, removal of a dissimilar image, candidate-set selection by visual characteristics, aggregation into one set | none | **outside as read (single source: Google Patents)**: image retrieval from visual annotations; `fuse` handles ids and scores only. Borda appears only in its description |
| US7188106B2 (re-read) | Expired - Lifetime | as in the table above | `Condorcet`, `Copeland` | status now read on the Google page: expired per Google; the verdict above stands; a second reader is still wanted |

## To do before row 6 can close
0. (Done 2026-11-26, Google Patents pages, single source: US5864845A claim 1 read in full: (a) train per engine by clustering training queries and building centroids, (b) weight each cluster by expected relevant pages, (c) select the nearest centroid for the incoming query and return its weight, (d) apportion the N result slots by those weights; every independent claim (1, 9) needs query clustering and training, which sortant lacks. US10698908B2 claims 4, 11 and 18 read: they only list CombSUM and CombMNZ among the retrieval models in a Markush group; claims 1, 8 and 15 still require the filter, segment, fuse and send-back steps. Both stay outside as read, single source: Google Patents.)
1. (Done 2026-11-26: US12259913 and US2025/0094398A1 read, both outside as read.) Ask a second reader to check the US7188106B2 reading;
   check the legal status of US11163780B2 and US7188106B2 on the USPTO. Read the later steps of US5864845A claim 1 and
   claims 4, 11 and 18 of US10698908B2 for completeness. None of the nine read so far reads on sortant as read.
2. Run the same queries on a second source (USPTO full text or Espacenet), each with a positive control.
3. (Done 2026-11-26, Google Patents only: classic-name queries run, see the section above; two hits, EP1631901A2 and US7716202B2, are marked "needs more".)
4. Never write "clear"; "outside as read" only, per patent, with the reason.

## EPO OPS comparison pilot, 2026-11-26: no comparison possible (coverage)

PM task: compare ten already-quoted claim sets against the EPO OPS text. Auth worked (token granted with the vault credentials,
nothing printed or written). Ten documents asked for the claims endpoint (`published-data/publication/docdb/<CC.NUM.KIND>/claims`):
US8838606B1, US8713023B1, US10698908B2, US8078606B2, US5864845A, US11163780B2, US7188106B2, US12259913B1, US2025/0094398A1
(pending application), EP1631901A2. Results: **0 of 10 have OPS claim text.** Nine US documents: HTTP 404
`CLIENT.InvalidCountryCode` ("unsupported country code ... fulltext"), so OPS serves no US claims at all. EP1631901 (A2, A3, B1 and
the epodoc form): HTTP 404 `SERVER.EntityNotFound`. Positive control, same token, same minute: EP1000000A1 returned 3469 bytes of
claims XML; an EP document OPS does not hold (EP3000000A1) returns the same 404 as EP1631901, so the 404 means "no fulltext held".
No foreign-language document is in the sweep, and no pending-application/grant pair (the Oracle application has not granted), so
those two requested cases were not available. The question "do reads from different bodies differ" stays unanswered: nothing was
compared. No earlier verdict is edited. Next: a pilot needs EP/WO documents (OPS fulltext) or a different second source for US text.
Tool defect found: `tools/patent-claims.py` counts claim blocks, not claims (US8838606B1 printed 1 block); the comparison tool
`tools/epo-ops-claims.py` (this branch) therefore needs per-claim splitting before a real diff on Google text.

### US text from the USPTO itself, 2026-11-26: every source answered, none gave text without a key

Order from the PM task, each tried keyless, exact answers:
1. **Bulk weekly grant XML.** `bulkdata.uspto.gov` no longer resolves (curl error 6). Its replacement `data.uspto.gov` serves a
   JavaScript app: every file path tried (`/bulkdata/datasets/ptgrxml/2014/ipg140916.zip`, `/ui/datasets/products/files/PTGRXML/2014/...`)
   returned the same 12258-byte HTML shell with HTTP 200, as did its own script bundle. The API behind it,
   `api.uspto.gov/api/v1/datasets/products/...`, answers HTTP 401 `{"message":"Unauthorized"}`. No file was downloaded.
2. **PatentsView.** `api.patentsview.org` and `patentsview.org` answer 301 to `data.uspto.gov/support/transition-guide/patentsview`;
   `search.patentsview.org` does not resolve. PatentsView now lives behind the same USPTO API, which wants a key.
3. **Patent Public Search.** `ppubs.uspto.gov/dirsearch-public/patents/US8838606B1`: HTTP 404; the app page loads but is the
   script-driven session app, not driven.
Blocker: a USPTO Open Data Portal API key (CireSnave or the PM would have to obtain it; I do not know its terms). Grant dates for the
first two: US8838606B1 2014-09-16 (Google page, "Application granted"). 0 of 10 US claims compared. Side note: Google's anticipated
expiry for US8838606B1 reads 2034-02-26 on today's page; the first table above says 2033-03-15 (earlier read); not re-derived.
Tool fix (portfolio `tools/`): `patent-claims.py` now splits per claim (`claims_from_page`; US8838606B1 = 18 claims, matching the
page's "Claims (18)"), with `tools/test_patent_claims.py` (4 tests, red first: 4 errors before the change, green after).

### Expiry recheck, read 2026-11-26 (expiry per Google; derived values drift with term adjustments)

The five rows that carry an expiry date, rechecked against the Google page in one pass: 2 of 5 changed, both the same family
(US8838606B1 and US8713023B1: 2033-03-15 to 2034-02-26); US10698908B2 (2037-10-18), US8078606B2 (2023-06-27) and US5864845A
(2016-06-28) unchanged. The earlier read of the family is not explained (a term adjustment is a guess, not checked). From now on
each expiry in these tables carries its read date.

## US second source: USPTO ODP own text vs Google (2026-11-26)

Method: `USPTO_ODP_API_KEY` via `with-secret` (one call, 86 API requests, 0 HTTP 429). Patent number to application number by ODP search
`applicationMetaData.patentNumber`; claims = the last `CLM` document before the Notice of Allowance, plus any examiner's amendment recorded in
the NOA. Limits found: the ODP claim text is the applicant's amendment listing, so **strikethrough and underline are lost** (deleted words
appear as live text, e.g. `between` next to `among`); older files are OCR (`fit tingig` for `(i) fitting`); older and some scanned documents
are image PDFs with no text layer and no OCR tool was available here. So "same" below means same after those artefacts, and anything the
markup would decide is "unresolved", not "same".

| Patent | Office text used | Result vs Google |
|---|---|---|
| US8838606B1 | CLM 2014-03-05 + examiner's amendment in NOA 2014-06-03 (claim 1) | 18 of 18 same; claim 1 matches Google only with the examiner's amendment applied (`among a plurality of rankings, wherein ...`, `or` to `and`); claims 2, 9, 11, 18 show `adapted` struck and `configured` inserted (markup artefact) |
| US8713023B1 | CLM 2014-02-26, no examiner's amendment | 20 of 20 same; claim 1 typo-level only (OCR `fit tingig`, `calculatin g` vs Google `(i) fitting`, `calculating`) |
| US8078606B2 | CLM 2011-10-25 (EXIN/NOA are image PDFs, unread) | list renumbered (office 21 claims, Google 17); 17 aligned by text; same except dependency numbers (renumbering) and `weighing` (office) vs `weighting` (Google) in claim 15; office claims 19-21 have no Google counterpart (unresolved) |
| US10698908B2 | CLM 2019-11-04, NOA has no amendment text | unresolved: office claim 1 carries limbs Google lacks (`digital input data comprising`, scores "associated with ... search terms ... received from a user who executes a ..."); consistent with struck text but not provable from this text; list also renumbered (26 vs 20) |
| US11163780B2 | CLM 2021-03-01 | unresolved: claim 4 office reads "at least one of, MaxPsg, CombMaxPsg, or PsgPFF", Google claim 4 reads "MaxPsg." (struck or not cannot be seen); claim 1 office has `maximal relevance` and claim 3 `is performed`, Google omits them |
| US12259913B1 | CLM 2024-09-03 | not compared: this file's claim layout was not parsed |
| US7188106B2, US7716202B2, US6728704B2, US7962500B2 | image PDFs | no text available (`pdftotext` returns 5-7 bytes; no OCR tool) |

Reading: no difference found that changes any verdict (none touches a limb the "outside" reasons rely on), but 3 of 10 are unresolved and 4 have
no text, so the second source covers 3 patents fully (US8838606B1, US8713023B1, US8078606B2 apart from its trailing claims). Verdicts stay "single source: Google Patents"
for the other seven. Next: an OCR tool for the image PDFs, or the issued-patent full text if the PM names a USPTO route that carries it.
