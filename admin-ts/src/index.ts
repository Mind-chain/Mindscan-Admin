import express from "express";
import AdminJS from "adminjs";
import AdminJSExpress from "@adminjs/express";
import path from "path";

import * as AdminJSPrisma from "@adminjs/prisma";
import JSONBig from "json-bigint";

import { componentLoader, Components } from "./components";
import {
  CreateSubmissionsResource,
  CreateTodoSubmissionsResource,
  CreateVerifiedAddressesResource,
  CreateTokenInfoResource,
  CreateUsersResource,
  CreateSuperSubmissionsResource,
} from "./resources";
import { CreateAuthOptions, CreateSessionOptions } from "./auth";

// DANGEROUSLY override JSON prototype methods to handle big ints.
JSON.parse = JSONBig.parse;
JSON.stringify = JSONBig.stringify;

AdminJS.registerAdapter({
  Resource: AdminJSPrisma.Resource,
  Database: AdminJSPrisma.Database,
});

const app = express();

// const isAdminRole = ({ currentAdmin }: { currentAdmin: CurrentAdmin }) => {
//     return currentAdmin && currentAdmin.role == "admin";
// };

// Run the server.
const run = async () => {
  // Very basic configuration of AdminJS

  const adminJs = new AdminJS({
    resources: [
      CreateSubmissionsResource(),
      CreateTodoSubmissionsResource(),
      CreateVerifiedAddressesResource(),
      CreateTokenInfoResource(),
      CreateUsersResource(),
      CreateSuperSubmissionsResource(),
    ],
    rootPath: "/admin", // Path to the AdminJS dashboard.
    componentLoader,
    branding: {
      companyName: "Blockscout Admin",
      logo: "/images/logo.svg",
      favicon: "/images/favicon.png",
      withMadeWithLove: false,
    },
    assets: {
      styles: ["/styles/global.css"],
    },
    dashboard: {
      component: Components.Dashboard,
    },
    env: {
      ADMIN_RS_HOST: process.env.ADMIN_RS_HOST,
    },
    locale: {
      language: "en",
      translations: {
        labels: {
          loginWelcome: "Login",
        },
        messages: {
          loginWelcome: "to Blockscout Admin Console",
        },
      },
    },
  });
  if (process.env.NODE_ENV === "production") await adminJs.initialize();
  else adminJs.watch();
  // Build and use a router to handle AdminJS routes.

  const router = AdminJSExpress.buildAuthenticatedRouter(
    adminJs,
    CreateAuthOptions(),
    null,
    CreateSessionOptions()
  );

  app.use(adminJs.options.rootPath, router);
  app.use(express.static(path.join(__dirname, "../public")));
  app.get("/", (_req, res) => res.status(301).redirect("/admin"));

  await app.listen(8080, () =>
    console.log(`Example app listening on port 8080!`)
  );
};

run();
