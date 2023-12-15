export const CreateAddressFilter = (fieldName) => (request, _context) => {
  const filterName = `filters.${fieldName}`;
  const { query = {} } = request;
  let address: string = query[filterName];
  if (address) {
    if (!address.startsWith("0x")) {
      address = "0x" + address;
    }
    query[filterName] = address.toLowerCase();
  }
  request.query = query;
  return request;
};

export const truncateString = (s: string, max_size: number) => {
  if (s && s.length > max_size) {
    return s.slice(0, max_size) + "...";
  } else {
    return s;
  }
};
