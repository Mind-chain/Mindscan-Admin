import { AdminDB } from "./models";

export const FetchUserChainIds = async (id: string) => {
  const user_chains = await AdminDB.usersChains.findMany({
    where: {
      user_id: BigInt(id),
    },
  });

  return user_chains.map((c) => c.chain_id);
};

export const SetUserChainIds = async (id: string, chain_ids: number[]) => {
  await AdminDB.usersChains.deleteMany({
    where: {
      user_id: BigInt(id),
    },
  });
  const usersChains = chain_ids.map((chain_id) => {
    return {
      user_id: BigInt(id),
      chain_id: chain_id,
    };
  });
  await AdminDB.usersChains.createMany({
    data: usersChains,
  });
};
