# Blockscout Admin Panel

Implementation of admin panel with [AdminJs](https://adminjs.co/). For now, it is just a prototype, to see some basic admin workflow.

## How to

1. Install required dependencies

```console
yarn install
```

1. Run postgres server and create service databases

```console
npx just start-postgres
npx just migrate-up
```

1. If any changes in database address, they can be updated via [`.env`](.env) file.
1. Generate Prisma clients and start the server (port 8080 is used by default; it is hardcoded inside [`src/index.ts`](src/index.ts)).

```console
yarn gstart
```

1. Create admin with cridentials `admin@blockscout.com:admin`. You can change it in UI after login.

```bash
docker exec -it admin-postgres psql -U postgres admin -c "INSERT INTO users (email, password, is_superuser) values ('admin@blockscout.com', '$2b$10$3tId5EFMmB91S0KyzXR7.eAe6JvahjH6Qsd7GnongQIVQhjI9whjC', true)"
```

1. Code linting

```console
yarn lint
```

## What to do

We need to write a component that allows to specify a reason why the application was rejected when clicking on `reject` button. The same way we need a component that allows to add a comment on what should be changed when `require updates` button is clicked.

The component for rejection has been started but contains just a template and is located at [reject-submission.tsx](src/components/reject-submission.tsx). It can be activated by removing `component: false,` and uncomennting `// component: Components.RejectSubmission, // TODO!!!` lines in [src/index.ts](src/index.ts).

Some instructions for writing the component into AdminJs - ["Writing your own Components"](https://docs.adminjs.co/ui-customization/writing-your-own-components)
