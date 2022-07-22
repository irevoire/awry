Are We Relevant Yet
===================

The purpose of this project is to measure the relevancy of multiple search engine.
To make it work you must index the [movies.json](movies.json) dataset in your engine.
Awry will then randomly select some documents in the datasets and do some search.

Then it will make a bunch of search on multiple criterion:
1. The classic search: It selects a part of the document and search it letter by letter.
2. Same with typo.
3. Same with missing words and wrong words in the middle or at the end.

For each of these criterion, awry will create the query letter by letter and only ask for the first 10 matches.
Awry will then give you point for all the requests where you successfuly found the document;
- If the document isn't found, you get 0 point.
- If the document is in the 10 matches you get 10 - match_idx points.

At the end awry will generate an array with the maximum theorical point you can get (this is impossible to reach)
and how many points you got in each category.
