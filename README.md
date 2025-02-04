# sidechain

Split and merge Unix pipelines for filtering and data manipulation.

## What is it?

`sidechain` is little cousin of `xargs` and `awk` that fills a gap in the standard
line-processing tool set.

It extends the reach of shell-based data processing just enough to make a 
difference for folks who still like to build load-bearing Unix pipelines.

### "Sidechain?"
The name and concept are borrowed from an [audio mixing
technique](https://www.sweetwater.com/insync/sidechaining-how-it-works-why-its-cool/)
in which an audio effect is controlled by a secondary audio signal.

![sidechain_audio](https://github.com/user-attachments/assets/e2892f11-163d-43b7-b289-855b3e57caf8)

For example, a compressor on a bass guitar might be controlled by a kick drum signal.
In this setup, the bass ducks out of the way on every kick drum hit, making for a
cleaner, punchier mix.

The side signal doesn't need to be from a different instrument. It could be a
filtered copy of the main signal. For example, a simple technique for [de-essing
vocals](https://en.wikipedia.org/wiki/De-essing#Side-chain_compression_or_broadband_de-essing)
is to use a compressor triggered by a high-pass-filtered copy of the main vocal
channel.

In a data pipeline, we can use `sidechain` to control our critical path using a side
command or pipeline.

Use cases of `sidechain` overlap with those of `xargs` or `awk`, but `sidechain`
has one key benefit: **it does not spawn a new process for every line of input**.

### Basic Example
Imagine we have lines of JSON-in-TSV:
```txt
# input.tsv
alice	{"foo":0,"bar":1}
billy	{"foo":1,"bar":1}
charlie	{"bar":0,"foo":1}
```
We want to filter this data to produce a list of users having `.foo != .bar`.
We could use `cut -f2 | jq 'select(.foo != .bar)'`, but then we'd lose the
usernames.

`sidechain` makes this easy. We can filter the data by using our `cut | jq` pipeline
as the side command, leaving the original lines intact:


```bash
$ cat input.tsv | sidechain filter -p true 'cut -f2 | jq ".foo != .bar"'
```

![sidechain_filter](https://github.com/user-attachments/assets/42c211b0-f426-40c5-ba51-13ad80512fed)

Arguments:
* `'cut -f2 | jq ".foo != bar"'`: The side command; this happens to output `true` when
  the condition is met.
* `-p true`: Retain only those lines whose side outputs match the pattern `true`.

Here, we're telling `sidechain` to start the side command, then pipe each line to
it and filter for the pattern `true`. Input lines that pass this test are emitted
**in their original, unmangled form.**

Important notes:
* The side command is spawned only once; it's a long-running subprocess that handles
  all input lines.
* When your input is provided via stdin, `sidechain` passes it on to the side command
  by default. You may not always want this; see the docs for how to control it
  explicitly.

## Map Mode
In map mode, your side command generates values which can be merged back into your
main pipeline.

### Example: Data Enrichment
Suppose you have a file containing lines of JSON with a `"url"` field, and you want
to extract the host component from each URL and add it as a field to each JSON
record.

```json
{"name":"alice","url":"https://foo.com"}
{"name":"billy","url":"http://1.2.3.4:8000/api"}
```

It's not hard to extract the host from a URL. But how would you surgically do it for
a URL embedded in JSON?

For simplicity, let's use an imaginary tool called `host-from-url` to extract the
hosts from the URLs. In reality, you could use the Ruby one-liner
`ruby -r uri -ne 'u = URI($_.chomp); puts(u.host || "")'`.

#### Solution with `sidechain`
We can use `-I` (like `xargs`) to define a placeholder character for our generated
values:

```bash
cat input.json | sidechain map -I% --side 'jq .url | host-from-url' jq '.host = "%"'
```
![sidechain_map](https://github.com/user-attachments/assets/ac0b3c50-2b7b-433e-9efc-5ebf4d827a91)

Here, the side command, `jq .url | host-from-url`, extracts the hosts, which are
then inserted back into the output of the main command, `jq '.host = "%"'`.

Remember, the side and main commands are each **spawned only once**.

## Using `$[]`
For cleaner, more-intuitive interpolation, you can use `$[]` to wrap your side
command:

```bash
cat input.json | sidechain map jq '.host = "$[jq .url | host-from-url]"'
```

This has the same behavior as the `-I%` version; it's just another way to spell it.

## Multiple Side Commands
Continuing with the URL-parsing example, imagine you want to not only extract the
host, but also the port:

```bash
cat input.json | sidechain map jq '
    .host = $[jq .url | host-from-url] |
    .port = $[jq .url | port-from-url]
  '
```
![sidechain_map_multiple](https://github.com/user-attachments/assets/a9707eb7-ee05-4c86-8c15-c427369729c1)

This is great, but it has a little bit of needless duplicated work: `jq .url` is run
twice in parallel.

To prevent this, you can insert a preliminary side command that feeds into your
downstream ones:
```bash
cat input.json | sidechain map \
  --side 'jq .url' \
  jq '.host = $[host-from-url] | .port = $[port-from-url]'
```
![sidechain_map_multiple_prelim](https://github.com/user-attachments/assets/91ec8f22-b350-4d7b-9b7e-077657ccb4ab)

