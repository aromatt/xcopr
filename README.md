# sidechain

Split and merge Unix pipelines for filtering and data manipulation.

## "Sidechain?"

The name and concept are borrowed from an audio mixing technique in which an audio
signal is processed using a device controlled by second audio signal.

For example, you might run a bass guitar signal through a compressor that is
triggered by a kick drum signal.

Similarly, in a Unix pipeline, we can use `sidechain` to control our critical path
using a second command.

## Filter Mode
Filter mode allows you to use a line-mangling filter while preserving the original,
unaltered input lines.

### Example
Imagine we have TSV data like this:
```txt
alice	{"foo":1,"bar":2}
robert	{"bar":2,"foo":2}
```
We want to find all users who have `.foo != .bar`.

It's easy enough to do `cut -f2 | jq 'select(.foo != .bar)'`, but this is no good
because `cut` removes the user names.

`sidechain` makes this easy:

```bash
$ cat input.tsv | sidechain filter 'cut -f2 | jq "select(.foo != .bar)"'

# output
alice	{"foo":1,"bar":2}
```

The command `cut -f2 | jq "select(.foo != .bar)"` is used as a filter. `sidechain`
emits the full, unmangled input lines which make it through the filter.

### A more realistic example
You have millions of files with names like `20231101_1.2.3.0-24.csv.gz`. The
`1.2.3.0-24` part represents an IP network (`1.2.3.0/24`). You have a list of
networks of interest, which may be child or parent networks of those in the
filenames. You want to produce a list of files whose CIDR overlaps with any in your
interesting list.

You have a tool called `filter-nets` that can efficiently filter a list of input
networks given a set of query networks. But in order to use it, you'd need to remove
the date prefixes and `.csv` suffixes, thereby mangling the filenames beyond repair.

`sidechain filter` is designed for this exact situation:
```bash
ls /path/to/files/ | sidechain filter 'sed <extract network> | filter-nets'
```

### Filtering ambiguity and performance
A typical filter introduces ambiguity: the number of output lines will be less than
or equal to the number of input lines, and there is no foolproof way to match up
input with output.

By default, `sidechain` uses a dynamic batching technique to solve this problem,
which requires invoking the filter command multiple times. This makes throughput
highly dependent on the input data.

To optimize performance, you can make your filter 1-to-1, meaning it will output
exactly one line per input line.

To use this optimization, you must tell `sidechain` about your commitment to make
your filter 1-to-1 using `-t <char>`.

Then, make sure your filter: (1) outputs exactly one line per input line and (2)
outputs `<char>` to indicate that a line has passed the filter.

`sidechain` will exclude from the final output all input lines for which the filter
produces anything other than `<char>`.

Here's the first example again, using `-t`:

```bash
cat input.tsv \
  | sidechain filter -t X 'cut -f2 | jq "if .foo != .bar then X else Y end"' \
  | cut -f1
```

## Map Mode
In map mode, your side command generates values (one per line) which are inserted
into lines of output.

We can use `-I` (like `xargs`) to define a placeholder character for our generated
values:

```bash
sidechain map -I% --side SIDE_CMD MAIN_CMD
```

Note: unlike filter mode, map mode introduces no ambiguity, so `sidechain` invokes
`SIDE_CMD` and `MAIN_CMD` exactly once each, processing all lines before exiting.

### Example: JSON clean-up
Suppose you have a file containing lines of JSON with an `"ip"` field, but some of
the "IPs" are actually URLs, and you want to clean up this data.

```json
{"name":"alice","date":"2024-02-01","ip":"9.8.7.6"}
{"name":"robert","date":"2024-01-01","ip":"http://1.2.3.4:8000/api"}
```

It's not too difficult to extract the host from a URL. But how would you surgically
do it for a URL embedded in JSON?

`sidechain map` makes this simple.

For simplicity, let's imagine you're using a tool called `host-from-url` to extract
the IPs from the URLs.

```bash
cat input.json | sidechain map -I% --side 'jq .ip | host-from-url' jq '.ip = "%"'
```

Here, the side command, `jq .ip | host-from-url`, extracts the IPs.

`sidechain` then inserts these values at the `%` for each line of output from the
main command, `jq '.ip = "%"'`.

Note that the side and main commands are each invoked _only once_ in map mode.
`sidechain` takes care of interpolating the output of the side command into the
output of the main command.

### Using `$[]`
For a cleaner, more-intuitive interpolation, you can use `$[]` to wrap your side
command:

```bash
cat input.json | sidechain map jq '.ip = "$[jq .ip | host-from-url]"'
```

This has the same behavior as the `-I%` version; `$[]` is just another way to write
it.

### Mapping from a file
If you have a file containing values to insert (as opposed to generating them on the
fly using a side-command), you may specify it with `-f`:

```bash
cat input.json | sidechain map -I% -f ips.txt jq '.ip = "%"'
```

Or, using `$[]`:
```bash
cat input.json | sidechain map '.ip = "$[cat ips.txt]"'
```

## Flatmap Mode
Flatmap mode is similar to map mode, but the side command can generate *multiple
values* per input line.

### Example
Consider a TSV file with this format:
```txt
1.2.3.4	{"users":[{"name":"foo"},{"name":"bar"}]}
```
You want to flatten it into:
```txt
1.2.3.4	{"name":"foo"}
1.2.3.4	{"name":"bar"}
```
We can use `sidechain flatmap`:

```bash
cat data.tsv | sidechain flatmap -I% \
  'cut -f2 | jq -c .cases[]' \
  awk '{print $1 "\t" "%"}'

```
Here, the side command `cut -f2 | jq -c ".users[]"` expands the `"users"` array into:

```txt
{"name":"foo"}
{"name":"bar"}
```
Then `sidechain` generates a final output line for each of these values by inserting
each value at the `%` in the main command, `awk '{print $1 "\t" "%"}'`.

You can use `$[]` in flatmap mode too:
```bash
cat data.tsv | sidechain flatmap awk '{print $1 "\t" "$[cut -f2 | jq -c .cases[]]"}'
```

### Using `$N[]`
For TSV data, you can use `$N[...]` as a shorthand for `$[cut -fN | ...]`:
```bash
cat data.tsv | sidechain flatmap '{print $1 "\t" "$2[jq -c .cases[]]}'
```
Note that `sidechain` replaces `$[]` and `$N[]` placeholders with other characters
when invoking the commands to avoid syntax conflicts with programs like `awk`.

### Flatmap n-to-m ambiguity
Note: because of the ambuity problem, flatmap mode must invoke the side command
separately for each input line.


