/// <reference no-default-lib="true" />
/// <reference lib="dom" />
/// <reference lib="dom.asynciterable" />
/// <reference lib="deno.ns" />
/// <reference lib="deno.unstable" />

/** @jsx h */
import { h } from "https://esm.sh/preact@10.8.1";
import { useState } from "https://esm.sh/preact@10.8.1/hooks";

export const getStaticProps: GetStaticProps<typeof Counter> = () => {
  return {
    name: "hey you!",
  };
};

export default function Counter(props: any) {
  return <div>Hey</div>;
}
