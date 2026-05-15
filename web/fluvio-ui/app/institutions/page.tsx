import type { Metadata } from "next";
import { InstitutionsPage } from "@/features/landing/components/InstitutionsPage";

export const metadata: Metadata = {
  title: "Institutions",
  description:
    "Satellite and aerial data, structured operations, governed multi-agent systems, and ML / deep learning / generative AI for food and water distribution.",
};

export default function InstitutionsMarketingPage() {
  return <InstitutionsPage />;
}
