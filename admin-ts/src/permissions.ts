import {
  ActionContext,
  ActionRequest,
  NotFoundError,
  CurrentAdmin,
} from "adminjs";
import { FetchUserChainIds } from "./chain_id";
import { AdminDB } from "./models";

// `before` hook
export const OnlyOwnerAccess = async (
  request: ActionRequest,
  context: ActionContext
) => {
  const { payload, method } = request;
  const { currentAdmin, record } = context;
  // if method is post, we should check payload of reqest,
  // if method is get, we should check requested record
  const data = method == "post" ? payload : record.params;
  if (!(await UserOwnsRecord(currentAdmin, data))) {
    throw new NotFoundError(
      "Not allowed to use this chain_id",
      "Action#handler"
    );
  }
  return request;
};

export const OnlySuperAdminAccess = ({ currentAdmin }: ActionContext) =>
  currentAdmin.is_superuser;

export const GetUserChains = async (currentAdmin: CurrentAdmin) => {
  if (currentAdmin.is_superuser) {
    // undefined means all chains
    return undefined;
  }
  return await FetchUserChainIds(currentAdmin.id);
};

const UserHasChainId = async (currentAdmin: CurrentAdmin, chain_id: bigint) => {
  const allowed = await GetUserChains(currentAdmin);
  // undefined means all chains
  if (allowed === undefined) {
    return true;
  } else {
    return allowed.includes(chain_id);
  }
};

const UserOwnsRecord = async (
  currentAdmin: CurrentAdmin,
  data: Record<string, any>
) => {
  let chain_id = undefined;

  // if record has chain_id directly
  if (data.chain_id) {
    chain_id = data.chain_id;

    // if it related to submission
  } else if (data.submission) {
    const submission_id = data.submission;
    chain_id = await AdminDB.submission
      .findUnique({
        where: {
          id: submission_id,
        },
      })
      .then((record) => record.chain_id);
  }

  if (chain_id != undefined) {
    return await UserHasChainId(currentAdmin, BigInt(chain_id));
  } else {
    // otherwise, this record isn't related to any chain and can be shown
    return true;
  }
};
