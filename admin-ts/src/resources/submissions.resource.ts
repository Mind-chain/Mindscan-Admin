import {
  ActionContext,
  ActionRequest,
  ActionResponse,
  ListActionResponse,
  RecordActionResponse,
  RecordJSON,
} from "adminjs";
import { Components } from "../components";
import { CreateOnlyOwnerListHandler } from "../handlers";
import { AdminDB, AdminModel } from "../models";
import { OnlyOwnerAccess, OnlySuperAdminAccess } from "../permissions";
import { CreateAddressFilter } from "../utils";
import { sendRecordToService } from "../contracts_info";
import {
  CREATE_CHAIN_ID_PRORERTY,
  ID_PROPERTY,
  ONLY_SHOW_PROPERTY,
  PROJECT_SECTOR_PROPERTY,
} from "../fields";
import { useNetworks } from "../networks";
import { FieldsNonEmptyValidatorBuilder } from "../validators";

const ShowAsUrl = {
  components: {
    show: Components.UrlPropery,
  },
};

const getAdminComments = async (record: RecordJSON) => {
  const id = record.id;
  let adminComments = "";

  if (record.params.status == "REJECTED") {
    const rejectedSubmission = await AdminDB.rejectedSubmission.findFirst({
      where: {
        submission_id: BigInt(id),
      },
      orderBy: {
        id: "desc",
      },
    });
    adminComments = rejectedSubmission?.reason;
  } else if (record.params.status == "WAITING_FOR_UPDATE") {
    const rejectedSubmission =
      await AdminDB.waitingForUpdateSubmission.findFirst({
        where: {
          submission_id: BigInt(id),
        },
        orderBy: {
          id: "desc",
        },
      });
    adminComments = rejectedSubmission?.admin_comments;
  }
  return adminComments;
};

const populateListWithAdminComments = async (
  response: ListActionResponse,
  _request: ActionRequest,
  _context: ActionContext
) => {
  for (let index = 0; index < response.records.length; index++) {
    const record = response.records[index];
    const adminComments = await getAdminComments(record);
    record.params.admin_comments = adminComments;
  }
  return response;
};

const populateRecordWithAdminComments = async (
  response: RecordActionResponse,
  _request: ActionRequest,
  _context: ActionContext
) => {
  const adminComments = await getAdminComments(response.record);
  response.record.params.admin_comments = adminComments;
  return response;
};

export const CreateSubmissionsResource = () => {
  const networks = useNetworks();

  return {
    resource: AdminModel("Submission"),
    options: {
      id: "allSubmissions",
      navigation: {
        name: "Token services",
      },
      sort: {
        sortBy: "updated_at",
        direction: "desc",
      },
      listProperties: [
        "id",
        "updated_at",
        "chain_id",
        "token_address",
        "status",
        "project_name",
        "blockscout_user_email",
      ],
      properties: {
        id: ID_PROPERTY,
        created_at: ONLY_SHOW_PROPERTY,
        updated_at: ONLY_SHOW_PROPERTY,
        token_address: {
          isDisabled: true,
        },
        blockscout_user_email: {
          isDisabled: true,
        },
        status: {
          isDisabled: true,
          components: {
            show: Components.Status.Show,
            list: Components.Status.List,
          },
        },
        chain_id: {
          ...CREATE_CHAIN_ID_PRORERTY(networks),
          isDisabled: true,
        },
        project_sector: PROJECT_SECTOR_PROPERTY,
        icon_url: {
          components: {
            show: Components.ImagePropery,
          },
        },
        project_website: ShowAsUrl,
        docs: ShowAsUrl,
        github: ShowAsUrl,
        telegram: ShowAsUrl,
        linkedin: ShowAsUrl,
        discord: ShowAsUrl,
        slack: ShowAsUrl,
        twitter: ShowAsUrl,
        open_sea: ShowAsUrl,
        facebook: ShowAsUrl,
        medium: ShowAsUrl,
        reddit: ShowAsUrl,
        coin_market_cap_ticker: ShowAsUrl,
        coin_gecko_ticker: ShowAsUrl,
        defi_llama_ticker: ShowAsUrl,
      },
      actions: {
        list: {
          handler: CreateOnlyOwnerListHandler(AdminDB, "chain_id"),
          before: [CreateAddressFilter("token_address")],
          after: [populateListWithAdminComments],
        },
        show: {
          before: [OnlyOwnerAccess],
          after: [populateRecordWithAdminComments],
        },
        new: {
          isAccessible: false,
        },
        delete: {
          isAccessible: OnlySuperAdminAccess,
        },
        bulkDelete: {
          isAccessible: false,
        },
        approve: {
          before: [OnlyOwnerAccess],
          actionType: "record",
          variant: "success",
          isAccessible: ({ record }) => {
            return record.param("status") === "IN_PROCESS";
          },
          component: false,
          handler: async (
            _request: ActionRequest,
            _response: ActionResponse,
            context: ActionContext
          ) => {
            const { record, currentAdmin } = await context;
            await sendRecordToService(record);
            record.set("status", "APPROVED");
            record.save(context);
            return {
              record: record.toJSON(currentAdmin),
              msg: "APPROVED",
            };
          },
        },
        reject: {
          before: [OnlyOwnerAccess],
          actionType: "record",
          variant: "danger",
          isAccessible: ({ record }) => {
            return record.param("status") === "IN_PROCESS";
          },
          component: Components.RejectSubmission,
          handler: (_request, _response, context) => {
            const { record, currentAdmin } = context;
            return {
              record: record.toJSON(currentAdmin),
            };
          },
        },
        mark_as_reject: {
          before: [OnlyOwnerAccess],
          actionType: "record",
          isVisible: false,
          handler: async (_request, _response, context) => {
            const { query } = _request;
            const { record, currentAdmin, h, resource } = context;
            const reason = query.reason;
            let msg = undefined;
            let redirectUrl = undefined;
            if (reason) {
              await AdminDB.rejectedSubmission.create({
                data: {
                  submission_id: record.get("id"),
                  reason: reason,
                },
              });
              record.set("status", "REJECTED");
              record.save(context);
              msg = "REJECTED";
              redirectUrl = h.resourceUrl({
                resourceId: resource.decorate().id(),
              });
            } else {
              msg = "REASON IS EMPTY";
            }
            return {
              record: record.toJSON(currentAdmin),
              msg: msg,
              redirectUrl: redirectUrl,
            };
          },
        },
        require_updates: {
          before: [OnlyOwnerAccess],
          actionType: "record",
          variant: "warn",
          isAccessible: ({ record }) => {
            return record.param("status") === "IN_PROCESS";
          },
          component: Components.RequireUpdate,
          handler: (_request, _response, context) => {
            const { record, currentAdmin } = context;
            return {
              record: record.toJSON(currentAdmin),
            };
          },
        },
        mark_as_require_updates: {
          before: [OnlyOwnerAccess],
          actionType: "record",
          isVisible: false,
          handler: async (request, _response, context) => {
            const { query } = request;
            const { record, currentAdmin, h, resource } = context;
            const adminComments = query.adminComments;
            let msg = undefined;
            let redirectUrl = undefined;
            if (adminComments) {
              await AdminDB.waitingForUpdateSubmission.create({
                data: {
                  submission_id: record.get("id"),
                  admin_comments: adminComments,
                },
              });
              record.set("status", "WAITING_FOR_UPDATE");
              record.save(context);
              msg = "WAITING_FOR_UPDATE";
              redirectUrl = h.resourceUrl({
                resourceId: resource.decorate().id(),
              });
            } else {
              msg = "COMMENT IS EMPTY";
            }
            return {
              record: record.toJSON(currentAdmin),
              msg: msg,
              redirectUrl: redirectUrl,
            };
          },
        },
      },
    },
  };
};
