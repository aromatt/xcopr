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
bob	{"foo":1}
alice	{"foo":0}
```
We want to print a list of all users that have `"foo" > 0`. How would you do it?

It's easy enough to do `cut -f2 | jq 'select(.foo > 0)'`, but this is no good because
`cut` removes the user names.

`sidechain` makes this easy:

```bash
cat input.tsv \
  | sidechain filter 'cut -f2 | jq "select(.foo > 0)"' \
  | cut -f1
```

Our side command, `cut -f2 | jq "select(.foo > 0)"`, is used as a filter. Lines that
pass the filter are then passed to the final output process, `cut -f1`.

### A more realistic example
You have millions of files with names like `20231101_1.2.3.0-24.csv.gz`. The
`1.2.3.0-24` part represents an IP network (`1.2.3.0/24`). You're interested in a
particular set of networks, but these might be child/parent networks of those in the
filenames.

You have a tool called `filter-nets` that can efficiently filter a list of input
networks given a set of query networks. But in order to use it, you'd need to remove
the date prefixes and `.csv` suffixes. This is a problem: you won't be able to
reconstruct the full filenames without those date prefixes.

```bash
ls /path/to/files/ | sidechain filter 'sed <extract network> | filter-nets'
```

### Improving performance with a 1-to-1 filter
Normally, a filter introduces ambiguity: the number of output lines will be less than
or equal to the number of input lines, and there is no foolproof way to match up
input with output, because the filter may modify each line (if it doesn't, then you
don't need to use `sidechain`!).

By default, `sidechain` uses a dynamic batching technique to solve this problem,
typically requiring multiple invocations of the filter command. This makes throughput
highly dependent on the input data.

To optimize performance, you can make your filter 1-to-1, meaning it will output
exactly one line per input line.

To declare this commitment, use `-t <char>`. Then, make sure your filter prints
exactly one line per input line; print `<char>` to indicate that a line has passed
the filter.

Here's the first example again, using `-t`:

```bash
cat input.tsv \
  | sidechain filter -t 1 'cut -f2 | jq "if .foo > 0 then 1 else 0 end"' \
  | cut -f1
```

## Map Mode
In map mode, your side command generates values (one per line) which are inserted
into lines of output.

We can use `-I` (like `xargs`) to define a placeholder character for our generated
values:

```bash
sidechain map -I% SIDE_COMMAND MAIN_COMMAND
```

Note: unlike filter mode, map mode introduces no ambiguity, so `sidechain` invokes
`SIDE_COMMAND` and `MAIN_COMMAND` exactly once each, processing all lines before
exiting.

### Example
Suppose you have a file containing lines of JSON with an `"ip"` field, but some of
the "IPs" are actually URLs. You want to clean up this data.

It's not too difficult to extract the host from a URL. But how would you reliably do
it for a field embedded within JSON?

`sidechain map` makes this simple.

For simplicity, let's imagine you're using a tool called `host-from-url` to extract
the hosts from the URLs.

```bash
cat input.json | sidechain map -I% 'jq .ip | host-from-url' jq '.ip = "%"'
```

### Using `$[]` for process substitution
For a cleaner, more-intuitive incantation, you can use `$[]` to wrap your side
command:

```bash
cat input.json | sidechain map jq '.ip = "$[jq .ip | host-from-url]"'
```

Remember: neither the main command nor the `$[]`-wrapped side command is invoked more
than once. All lines are processed using a single invocation.

### Mapping from a file
Alternatively, you can provide a file containing the values that you want to insert:
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
Consider a file with this format:
```txt
1.2.3.4	{"users":[{"name":"foo"},{"name":"bar"}]}
```

You want to flatten it into:
```txt
1.2.3.4	{"name":"foo"}
1.2.3.4 {"name":"bar"}
```

We can use flatmap mode:

```bash
cat input.tsv | sidechain flatmap -I% 'jq -c ".users[]"' awk '{print $1 "\t" "%"}'
```

`$[]` works too:
```bash
cat input.tsv | sidechain flatmap awk '{print $1 "\t" "$[cut -f2 | jq -c \".users[]\"]"}'
```

Note: because of the ambuity problem, flatmap mode invokes the side command
separately for each input line.
