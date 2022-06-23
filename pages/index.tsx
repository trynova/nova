/** @jsx h */
import { h } from "https://unpkg.com/preact?module";
import htm from "https://unpkg.com/htm?module";

const html = htm.bind(h) as any;

type NovaCtx = {};

type GetStaticProps<Component extends (props: any) => any> = (
  ctx: NovaCtx
) => Parameters<Component>[0];

export const getStaticProps: GetStaticProps<typeof HomePage> = (ctx) => {
  return { name: "hey" };
};

type a = Parameters<typeof HomePage>[0];

export interface HomePageProps {
  name: string;
}

export default function HomePage(props: HomePageProps) {
  return <h1>Hello, world!</h1>;
}

console.log(HomePage.toString());
