import { convertFilter } from "@adminjs/prisma";
import { PrismaClient } from "@prisma/client";
import {
  ActionContext,
  BaseRecord,
  flat,
  ActionQueryParameters,
  Filter,
  populator,
  SortSetter,
  AppError,
  BaseResource,
} from "adminjs";
import { GetUserChains } from "./permissions";

const PER_PAGE_LIMIT = 500;

// COPY OF
// https://github.com/SoftwareBrothers/adminjs/blob/e04003227142465a9a3fcf393c6323e25a16149a/src/backend/actions/list/list-action.ts#L32
// WITH CUSTOM PRISMA FILTER
export const CreateOnlyOwnerListHandler =
  (Database: PrismaClient, chain_id_field: string) =>
  async (request, response, context: ActionContext) => {
    const { query } = request;
    const {
      sortBy,
      direction,
      filters = {},
    } = flat.unflatten(query || {}) as ActionQueryParameters;
    const { resource, _admin } = context;

    let { page, perPage } = flat.unflatten(
      query || {}
    ) as ActionQueryParameters;

    if (perPage) {
      perPage = +perPage > PER_PAGE_LIMIT ? PER_PAGE_LIMIT : +perPage;
    } else {
      perPage = _admin.options.settings?.defaultPerPage ?? 10;
    }
    page = Number(page) || 1;

    const listProperties = resource.decorate().getListProperties();
    const firstProperty = listProperties.find((p) => p.isSortable());
    let sort;
    if (firstProperty) {
      sort = SortSetter(
        { sortBy, direction },
        firstProperty.name(),
        resource.decorate().options
      );
    }

    const filter = await new Filter(filters, resource).populate(context);
    const { currentAdmin } = context;
    const allowed_chain_ids = await GetUserChains(currentAdmin);
    const { records, count } = await find(
      Database,
      chain_id_field,
      resource,
      filter,
      perPage,
      page,
      sort,
      allowed_chain_ids
    );
    const populatedRecords = await populator(records, context);

    // eslint-disable-next-line no-param-reassign
    context.records = populatedRecords;

    return {
      meta: {
        total: count,
        perPage,
        page,
        direction: sort?.direction,
        sortBy: sort?.sortBy,
      },
      records: populatedRecords.map((r) => r.toJSON(currentAdmin)),
    };
  };

const find = async (
  Database: PrismaClient,
  chain_id_field: string,
  resource: BaseResource,
  filter: Filter,
  perPage: number,
  page: number,
  sort: any,
  allowed_chain_ids: bigint[]
) => {
  const Model = Database[resource.id()];
  const prisma_resource = resource as any;

  const limit = perPage;
  const offset = (page - 1) * perPage;
  const { direction, sortBy } = sort;

  const where = buildPrismaFilter(
    prisma_resource,
    chain_id_field,
    filter,
    allowed_chain_ids
  );

  const records = await Model.findMany({
    where,
    skip: offset,
    take: limit,
    orderBy: {
      [sortBy]: direction,
    },
  });
  const base_records: BaseRecord[] = records.map(
    (record) =>
      new BaseRecord(
        prisma_resource.prepareReturnValues(record),
        prisma_resource
      )
  );

  const count = await Model.count({ where });
  return { records: base_records, count: count };
};

const buildPrismaFilter = (
  prisma_resource: any,
  chain_id_field: string,
  filter: Filter,
  allowed_chain_ids: bigint[]
) => {
  const converted_filters = convertCaseInsensitiveFilter(
    prisma_resource.model.fields,
    filter
  );
  // indefined means all chains
  if (allowed_chain_ids != undefined) {
    if (chain_id_field == "chain_id") {
      converted_filters["chain_id"] = {
        in: allowed_chain_ids,
      };
    } else if (chain_id_field == "submission.chain_id") {
      converted_filters["submission"] = {
        chain_id: { in: allowed_chain_ids },
      };
    } else {
      throw new AppError(`unknow chain_id field: ${chain_id_field}`);
    }
  }
  return converted_filters;
};

const convertCaseInsensitiveFilter = (modelFields, filterObject) => {
  const filter = convertFilter(modelFields, filterObject);
  for (const name in filter) {
    if (filter[name].contains) {
      filter[name].mode = "insensitive";
    }
  }
  return filter;
};
