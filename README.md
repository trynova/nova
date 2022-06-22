> Nova is in it's youth and not for use yet, this repository is just
> for a more streamlined developoment cycle.

# A Deno-first performant resource compiler

Nova leverages Zig, a low-level general-purpose language largely-heiled as a C
killer, to build high-performance parsers to generate the JavaScript files from
your resources.

## How does it work?

Nova builds JavaScript files for your resource files (e.g. Markdown, Mustache
templates, ect.) and then updates your import map to make the raw resource file
point to the compiled JavaScript file in our custom `_resources` folder.

## Setup

### The `/pages` directory

All of your pages should be located in the `/pages` directory. Nova utilizes
file-based routing in which the file names correspond to routes. It also
supports dynamic parameters in the file names which will then be injected into
the page. For example, let's say you have the following Mustache template
located at `/pages/hello-[name].mustache`:

```svelte
Hello {{ name }}, how are you?
```

You will notice that we did not pass in any props to this template. Well, by
default, Nova will inject all undefined props that are in the URL to the
template.

<!-- TODO: Comment about static analysis  -->

## Notes

- None of the parsers currently support source map modes, but this is planned.
- The Mustache parser does not support custom delimiters.
  [See the Mustache(5) reference.](https://mustache.github.io/mustache.5.html#Set-Delimiter)

## License

[MIT](./LICENSE)
