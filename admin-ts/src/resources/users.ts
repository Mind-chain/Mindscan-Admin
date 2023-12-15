import { AdminModel } from "../models";
import { useNetworks } from "../networks";
import { hash } from "bcrypt";
import {
  ListActionResponse,
  RecordActionResponse,
  ResourceWithOptions,
  ValidationError,
  ActionRequest,
  PropertyErrors,
  ActionContext,
  NotFoundError,
} from "adminjs";
import { GetUserChains, OnlySuperAdminAccess } from "../permissions";
import { Components } from "../components";
import { FetchUserChainIds, SetUserChainIds } from "../chain_id";
import { ID_PROPERTY, ONLY_SHOW_PROPERTY } from "../fields";

const validateForm = (request: ActionRequest) => {
  // We only want to validate "post" requests
  if (request.method !== "post") return request;

  const errors: PropertyErrors = {};
  const chain_ids = getArrayFromMultiPartRequest(request.payload, "chain_ids");
  if (chain_ids.length == 0 && !request.payload?.is_superuser) {
    errors.chain_ids = {
      message: "Choose at least one chain",
    };
  }

  if (Object.keys(errors).length) {
    throw new ValidationError(errors);
  }

  return request;
};

const hashPassword = async (request: ActionRequest) => {
  // no need to hash on GET requests, we'll remove passwords there anyway
  if (request.method !== "post") return request;

  if (request.payload?.password) {
    request.payload.password = await hash(request.payload.password, 10);
  } else {
    delete request.payload?.password;
  }

  return request;
};

const getArrayFromMultiPartRequest = (
  payload: Record<string, any>,
  arrayName: string
) => {
  if (payload[arrayName] !== undefined) {
    return payload[arrayName];
  } else {
    return Object.entries(payload)
      .filter(([key, _]: [string, any]) => key.startsWith(arrayName))
      .map(([_, value]) => value);
  }
};

const updateRecordChainIds = async (
  response: RecordActionResponse,
  request: ActionRequest,
  context: ActionContext
) => {
  if (request.method !== "post") return response;
  const chain_ids: number[] = getArrayFromMultiPartRequest(
    request.payload,
    "chain_ids"
  ).map(Number);
  const { record } = context;
  await SetUserChainIds(record.params.id, chain_ids);
  return response;
};

const populateRecordChainIds = async (
  response: RecordActionResponse,
  request: ActionRequest,
  context: ActionContext
) => {
  if (request.method !== "get") return response;
  const { record } = context;
  const chain_ids = await FetchUserChainIds(record.params.id);
  response.record.params.chain_ids = chain_ids;
  return response;
};

const populateRecordsChainIds = async (
  response: ListActionResponse,
  request: ActionRequest,
  _context: ActionContext
) => {
  if (request.method !== "get") return response;
  for (let i = 0; i < response.records.length; i++) {
    const record = response.records[i];
    const chain_ids = await FetchUserChainIds(record.params.id);
    record.params.chain_ids = chain_ids;
  }
  return response;
};

const hideRecordPassword = async (response: RecordActionResponse) => {
  response.record.params.password = "";
  return response;
};

const hideRecordsPassword = async (response: ListActionResponse) => {
  response.records.forEach((record) => {
    record.params.password = "";
  });
  return response;
};

const onlyCurrentAdminInListFilter = (
  request: ActionRequest,
  context: ActionContext
) => {
  if (OnlySuperAdminAccess(context)) {
    return request;
  }
  const { query = {} } = request;
  const newQuery = {
    ...query,
    ["filters.id"]: context.currentAdmin?.id,
  };

  request.query = newQuery;
  return request;
};

const OnlySuperUserOrOwnerAccess = (context: ActionContext) => {
  const superAdmin = OnlySuperAdminAccess(context);
  const owner = context.record?.id() == context.currentAdmin?.id;
  return Boolean(superAdmin) || owner;
};

export const CreateUsersResource = (): ResourceWithOptions => {
  const networks = useNetworks();

  return {
    resource: AdminModel("User"),
    options: {
      id: "Users",
      navigation: {
        name: "Super User",
      },
      properties: {
        id: ID_PROPERTY,
        created_at: ONLY_SHOW_PROPERTY,
        password: {
          isVisible: {
            list: false,
            filter: false,
            show: false,
            edit: true, // we only show it in the edit view
          },
          isRequired: false,
        },
        chain_ids: {
          type: "string",
          isArray: true,
          components: {
            edit: Components.ChainIds.Select,
            show: Components.ChainIds.Show,
            list: Components.ChainIds.List,
            filter: Components.ChainIds.Filter,
          },
          props: {
            networks,
          },
          isVisible: {
            filter: false,
            list: true,
            show: true,
            edit: true,
          },
        },
        // submissions: {
        //     reference: "Submissions"
        // },
      },
      actions: {
        new: {
          isAccessible: OnlySuperAdminAccess,
          before: [hashPassword, validateForm],

          after: [updateRecordChainIds],
        },
        edit: {
          isAccessible: OnlySuperAdminAccess,
          before: [
            hashPassword,
            // validateForm
          ],
          after: [
            hideRecordPassword,
            populateRecordChainIds,
            updateRecordChainIds,
          ],
        },
        show: {
          isAccessible: OnlySuperUserOrOwnerAccess,
          after: [hideRecordPassword, populateRecordChainIds],
        },
        list: {
          before: [onlyCurrentAdminInListFilter],
          after: [hideRecordsPassword, populateRecordsChainIds],
        },

        availableNetworks: {
          isVisible: false,
          actionType: "resource",
          component: false,
          handler: async (_request, _response, context) => {
            const { currentAdmin } = context;
            const networks = await GetUserChains(currentAdmin);
            return {
              networks: networks,
            };
          },
        },
        delete: {
          isAccessible: OnlySuperAdminAccess,
        },
        changePassword: {
          isAccessible: OnlySuperUserOrOwnerAccess,
          actionType: "record",
          icon: "Key",
          handler: async (request, response, context) => {
            const { record, resource, currentAdmin, h } = context;
            if (!record) {
              throw new NotFoundError(
                [
                  `Record of given id ("${request.params.recordId}") could not be found`,
                ].join("\n"),
                "Action#handler"
              );
            }
            if (request.method === "get") {
              return { record: record.toJSON(currentAdmin) };
            }
            if (request.method === "post") {
              const { record } = context;
              const { newPassword } = request.payload ?? {};
              if (!newPassword) {
                throw new ValidationError({
                  newPassword: { message: "invalid password" },
                });
              }
              record.params.password = await hash(newPassword, 10);
              await record.save();
              return {
                record: record.toJSON(currentAdmin),
                redirectUrl: h.resourceUrl({
                  resourceId: resource._decorated?.id() || resource.id(),
                }),
                notice: {
                  message: "Password changed successfully",
                  type: "success",
                },
              };
            }
            return { record: record.toJSON(currentAdmin) };
          },
          component: Components.ChangePassword,
        },
      },
    },
  };
};
