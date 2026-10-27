import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

import type { Capabilities, ImageOrientation, ImagePageSize, Inspection, OperationResult, Preset } from "./types";

const pdfFilter = [{ name: "PDF documents", extensions: ["pdf"] }];
const imageFilter = [{ name: "Images", extensions: ["jpg", "jpeg", "jfif", "jfi", "jif", "png", "webp"] }];

export async function choosePdf(): Promise<string | null> {
  const selected = await open({ multiple: false, directory: false, filters: pdfFilter });
  return typeof selected === "string" ? selected : null;
}

export async function choosePdfs(): Promise<string[]> {
  const selected = await open({ multiple: true, directory: false, filters: pdfFilter });
  if (!selected) return [];
  return Array.isArray(selected) ? selected : [selected];
}

export async function chooseImages(): Promise<string[]> {
  const selected = await open({ multiple: true, directory: false, filters: imageFilter });
  if (!selected) return [];
  return Array.isArray(selected) ? selected : [selected];
}

export async function chooseOutput(defaultPath: string): Promise<string | null> {
  return save({ defaultPath, filters: pdfFilter });
}

export function inspectFile(path: string): Promise<Inspection> {
  return invoke("inspect_file", { path });
}

export function getCapabilities(): Promise<Capabilities> {
  return invoke("capabilities");
}

export function compressPdf(request: {
  input: string;
  output: string;
  preset: Preset;
  targetBytes: number | null;
  overwrite: boolean;
  allowSignatureInvalidation: boolean;
}): Promise<OperationResult> {
  return invoke("compress_pdf", { request });
}

export function mergePdfs(request: {
  inputs: string[];
  output: string;
  overwrite: boolean;
  allowSignatureInvalidation: boolean;
  allowStructureLoss: boolean;
}): Promise<OperationResult> {
  return invoke("merge_pdfs", { request });
}

export function imagesToPdf(request: {
  inputs: string[];
  output: string;
  overwrite: boolean;
  pageSize: ImagePageSize;
  orientation: ImageOrientation;
  marginMm: number;
  dpi: number;
  recompressQuality: number;
}): Promise<OperationResult> {
  return invoke("images_to_pdf", { request });
}
