import { ParamDefinition } from "../prompts/types";

/**
 * Program command parameter definitions
 */

export const extendProgramParams: ParamDefinition[] = [
    {
        name: "bytes",
        type: "amount",
        description: "Additional bytes to allocate",
        required: true,
        flag: "--bytes",
        shortFlag: "-b",
    },
];
