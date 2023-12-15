import React, { useState } from "react";
import {
  Box,
  H3,
  Input,
  Button,
  DrawerContent,
  DrawerFooter,
} from "@adminjs/design-system";
import { ActionProps } from "adminjs";
import { ApiClient } from "adminjs";
import { useNavigate } from "react-router";

const api = new ApiClient();

const RejectSubmission = (props: ActionProps) => {
  const { record: initialRecord, resource, action } = props;
  const navigate = useNavigate();
  const [reason, setReason] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (reason.length > 0) {
      console.log("set reject status to submission");
      setLoading(true);
      api
        .recordAction({
          recordId: initialRecord.id,
          resourceId: resource.id,
          actionName: "mark_as_reject",
          params: {
            reason: reason,
          },
        })
        .then((response) => {
          if (response.data.redirectUrl) {
            navigate(response.data.redirectUrl);
          }
        })
        .finally(() => {
          setLoading(false);
        });
    }
  };

  return (
    <Box as="form" flex onSubmit={handleSubmit}>
      <Box
        variant="white"
        width={1 / 2}
        boxShadow="card"
        mr="xxl"
        flexShrink={0}
      >
        <DrawerContent>
          <H3>Please, specify the rejection reason</H3>
          <Input
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            placeholder="Reject reason"
            width={1}
            variant="xl"
          ></Input>
        </DrawerContent>
        <DrawerFooter>
          <Button type="submit" variant="danger" size="lg" disabled={loading}>
            Reject
          </Button>
        </DrawerFooter>
      </Box>
    </Box>
  );
};

export default RejectSubmission;
