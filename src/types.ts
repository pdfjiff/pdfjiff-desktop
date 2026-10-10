export type Tool = "compress" | "merge" | "inspect" | "capabilities";

export interface PageInfo {
  number: number;
  widthPt: number;
  heightPt: number;
  rotation: number;
}

export interface Inspection {
  path: string;
  fileName: string;
  inputBytes: number;
  pageCount: number | null;
  pages: PageInfo[];
  encrypted: boolean;
  signaturesDetected: boolean;
  assemblySensitiveFeatures: string[];
}

export interface Capability {
  id: string;
  label: string;
  description: string;
  available: boolean;
}

export interface Capabilities {
  appVersion: string;
  schemaVersion: number;
  offline: boolean;
  available: Capability[];
  planned: Capability[];
}

export interface OperationResult {
  operation: "pdf.compress" | "pdf.merge";
  output: string;
  inputBytes: number;
  outputBytes: number;
  pageCount: number;
  elapsedMs: number;
  savedRatio: number | null;
  attempts: number | null;
  imagesRecompressed: number | null;
  imagesDownsampled: number | null;
  warnings: string[];
}

export type Preset = "quality" | "balanced" | "small" | "lossless";
