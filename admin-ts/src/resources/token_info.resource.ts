import { CreateAddressFilter } from "../utils";
import { Components } from "../components";
import { sendRecordToService } from "../contracts_info";
import { CreateOnlyOwnerListHandler } from "../handlers";
import { ContractsInfoDB, ContractsInfoModel } from "../models";
import { OnlyOwnerAccess } from "../permissions";
import {
  FieldsNonEmptyValidatorBuilder,
  UniqueChainIdAddressValidator,
} from "../validators";
import {
  Action,
  ActionContext,
  NotFoundError,
  NotImplementedError,
  RecordActionResponse,
  paramConverter,
} from "adminjs";
import {
  CREATE_CHAIN_ID_PRORERTY,
  ID_PROPERTY,
  ONLY_SHOW_PROPERTY,
  PROJECT_SECTOR_PROPERTY,
} from "../fields";
import { useNetworks } from "../networks";

const ShowAsUrl = {
  components: {
    show: Components.UrlPropery,
  },
};

const EditDeleteAreVisible = (context: ActionContext) => {
  // TODO: return false if current action is duplicate.
  // problem is that context.action.name contains 'edit' or 'delete' value, not 'duplicate'
  return true;
};

export const CreateTokenInfoResource = () => {
  const networks = useNetworks();
  return {
    resource: ContractsInfoModel("TokenInfo"),
    options: {
      id: "tokenInfos",
      navigation: {
        name: "Token services",
      },
      sort: {
        sortBy: "created_at",
        direction: "desc",
      },
      listProperties: [
        "chain_id",
        "address",
        "project_name",
        "project_email",
        "project_website",
        "project_sector",
      ],
      showProperties: [
        "chain_id",
        "address",
        "project_name",
        "project_website",
        "project_email",
        "token_name",
        "token_symbol",
        "icon_url",
        "project_sector",
        "project_description",
        "docs",
        "github",
        "telegram",
        "linkedin",
        "discord",
        "slack",
        "twitter",
        "open_sea",
        "facebook",
        "medium",
        "reddit",
        "support",
        "coin_market_cap_ticker",
        "coin_gecko_ticker",
        "defi_llama_ticker",
        "is_user_submitted",
        "id",
        "created_at",
      ],
      properties: {
        id: ID_PROPERTY,
        created_at: ONLY_SHOW_PROPERTY,
        is_user_submitted: ONLY_SHOW_PROPERTY,
        icon_url: {
          components: {
            show: Components.ImagePropery,
            edit: Components.IconUrl.Edit,
          },
        },
        project_sector: PROJECT_SECTOR_PROPERTY,
        chain_id: {
          ...CREATE_CHAIN_ID_PRORERTY(networks),
          isDisabled: {
            edit: true,
            new: false,
          },
        },
        address: {
          isDisabled: {
            edit: true,
            new: false,
          },
          components: {
            edit: Components.DisabledEditText,
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
          handler: CreateOnlyOwnerListHandler(ContractsInfoDB, "chain_id"),
          before: [CreateAddressFilter("address")],
        },
        show: {
          before: [OnlyOwnerAccess],
        },
        delete: {
          isVisible: EditDeleteAreVisible,
          before: [OnlyOwnerAccess],
          guard:
            "Are you sure? (this record won't be deleted from blockscout instance)",
        },
        new: NewAction,
        edit: EditAction,
        bulkDelete: {
          isAccessible: false,
        },
        duplicate: DuplicateAction,
      },
    },
  };
};

const NewAction: Action<RecordActionResponse> = {
  name: "new",
  actionType: "resource",
  before: [
    OnlyOwnerAccess,
    FieldsNonEmptyValidatorBuilder(["chain_id"]),
    UniqueChainIdAddressValidator,
  ],
  handler: async (request, _response, context) => {
    const { resource, currentAdmin, h, translateMessage } = context;
    const params = paramConverter.prepareParams(
      request.payload ?? {},
      resource
    );
    const record = await resource.build(params);
    await sendRecordToService(record);
    return {
      record: record.toJSON(currentAdmin),
      redirectUrl: h.resourceUrl({
        resourceId: resource._decorated?.id() || resource.id(),
      }),
      notice: {
        message: translateMessage("successfullyCreated", resource.id()),
        type: "success",
      },
    };
  },
};

const EditAction: Action<RecordActionResponse> = {
  name: "edit",
  actionType: "record",
  before: [OnlyOwnerAccess, FieldsNonEmptyValidatorBuilder(["chain_id"])],
  isVisible: EditDeleteAreVisible,
  handler: async (request, _response, context) => {
    const { record, resource, currentAdmin, h, translateMessage } = context;
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
    } else {
      const params = paramConverter.prepareParams(
        request.payload ?? {},
        resource
      );
      const newRecord = await record.update(params, context);
      await sendRecordToService(newRecord);
      return {
        redirectUrl: h.resourceUrl({
          resourceId: resource._decorated?.id() || resource.id(),
        }),
        record: record.toJSON(currentAdmin),
        notice: {
          message: translateMessage("successfullyUpdated", resource.id()),
          type: "success",
        },
      };
    }
  },
};

const DuplicateAction: Action<RecordActionResponse> = {
  name: "duplicate",
  isVisible: true,
  actionType: "record",
  icon: "Pen",
  handler: async (request, response, context) => {
    const { record, currentAdmin } = context;
    if (!record) {
      throw new NotFoundError(
        [
          `Record of given id ("${request.params.recordId}") could not be found`,
        ].join("\n"),
        "Action#handler"
      );
    }
    if (request.method === "get") {
      const recordJson = record.toJSON(currentAdmin);
      // remove some fields since some of them will be passed to the "new" component
      recordJson.params.id = null;
      recordJson.params.address = null;
      recordJson.params.chain_id = null;
      return { record: recordJson };
    } else if (request.method === "post") {
      // since component should send POST request to 'new' action
      throw new NotImplementedError("should be unreachable");
    }
  },
  component: Components.Duplicate,
};
