// Writes every call site in order, for matching our functions against other
// builds by call graph (tools/cg-match.py):
//   <out>/call_sites.tsv   caller, call-site address, callee
// Calls through pointers have no callee and are left out.
//
// Script argument: absolute output directory.
//@category FFX

import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;

import java.io.BufferedWriter;
import java.nio.file.Files;
import java.nio.file.Path;

public class DumpCallSites extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) {
            throw new IllegalArgumentException("usage: DumpCallSites <output dir>");
        }
        Path out = Path.of(args[0]);
        Files.createDirectories(out);
        FunctionManager fm = currentProgram.getFunctionManager();
        long rows = 0;
        try (BufferedWriter w = Files.newBufferedWriter(out.resolve("call_sites.tsv"))) {
            w.write("caller\tsite\tcallee\n");
            for (Function f : fm.getFunctions(true)) {
                if (f.isExternal() || f.isThunk()) {
                    continue;
                }
                InstructionIterator it = currentProgram.getListing().getInstructions(f.getBody(), true);
                while (it.hasNext()) {
                    Instruction ins = it.next();
                    if (!ins.getFlowType().isCall()) {
                        continue;
                    }
                    for (Reference r : ins.getReferencesFrom()) {
                        if (!r.getReferenceType().isCall()) {
                            continue;
                        }
                        Function c = fm.getFunctionAt(r.getToAddress());
                        if (c == null) {
                            continue;
                        }
                        if (c.isThunk() && c.getThunkedFunction(true) != null) {
                            c = c.getThunkedFunction(true);
                        }
                        w.write("0x" + f.getEntryPoint() + "\t0x" + ins.getAddress() + "\t0x"
                                + c.getEntryPoint() + "\n");
                        rows++;
                    }
                }
            }
        }
        println("DumpCallSites: wrote " + rows + " rows");
    }
}
