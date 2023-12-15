import { CreateAddressFilter } from "../utils";
import { CreateSubmissionsResource } from "./submissions.resource";

const onlyInProcess = (request, context) => {
  const { query = {} } = request;
  const newQuery = {
    ...query,
    ["filters.status"]: "IN_PROCESS",
  };
  request.query = newQuery;
  return request;
};

export const CreateTodoSubmissionsResource = () => {
  const submissions_resource = CreateSubmissionsResource();
  return {
    ...submissions_resource,
    options: {
      ...submissions_resource.options,
      id: "todoSubmissions",
      sort: {
        sortBy: "created_at",
        direction: "asc",
      },
      actions: {
        ...submissions_resource.options.actions,
        list: {
          ...submissions_resource.options.actions.list,
          before: [onlyInProcess, CreateAddressFilter("token_address")],
        },
      },
    },
  };
};
