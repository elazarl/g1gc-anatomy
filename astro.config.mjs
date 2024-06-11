import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// https://astro.build/config
export default defineConfig({
  integrations: [
    starlight({
      title: "G1GC",
      editLink: {
        baseUrl: "https://github.com/elazarl/g1gc-anatomy/edit/master/",
      },
      social: {
        github: "https://github.com/elazarl/g1gc-anatomy",
      },
      sidebar: [
        /*{
          label: "Guides",
          items: [
            // Each item here is one entry in the navigation menu.
            { label: "Example Guide", link: "/guides/young/" },
          ],
        },*/
        {
          label: "G1GC Simple",
          autogenerate: { directory: "simple" },
        },
      ],
    }),
  ],
});
