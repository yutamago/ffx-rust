// Writes which functions touch which global data, for the ownership map
// (docs/rewrite-plan.md, M0):
//   <out>/data_refs.tsv   function, data address, access, count
// access: R read, W write, A address taken (pointer to it computed or passed).
// Only references into initialized or uninitialized data blocks count; code
// addresses (calls, jumps, jump tables) are skipped.
//
// Script argument: absolute output directory.
//@category FFX

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.RefType;
import ghidra.program.model.symbol.Reference;

import java.io.BufferedWriter;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import java.util.TreeMap;

public class DumpDataRefs extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) {
            throw new IllegalArgumentException("usage: DumpDataRefs <output dir>");
        }
        Path out = Path.of(args[0]);
        Files.createDirectories(out);
        long rows = 0;
        try (BufferedWriter w = Files.newBufferedWriter(out.resolve("data_refs.tsv"))) {
            w.write("function\taddress\taccess\tcount\n");
            for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
                if (f.isExternal() || f.isThunk()) {
                    continue;
                }
                Map<String, Integer> seen = new TreeMap<>();
                InstructionIterator it = currentProgram.getListing().getInstructions(f.getBody(), true);
                while (it.hasNext()) {
                    Instruction ins = it.next();
                    for (Reference r : ins.getReferencesFrom()) {
                        Address to = r.getToAddress();
                        if (!to.isMemoryAddress()) {
                            continue;
                        }
                        MemoryBlock b = currentProgram.getMemory().getBlock(to);
                        if (b == null || b.isExecute()) {
                            continue;
                        }
                        RefType t = r.getReferenceType();
                        String access;
                        if (t.isWrite()) {
                            access = "W";
                        } else if (t.isRead()) {
                            access = "R";
                        } else if (t.isData()) {
                            access = "A";
                        } else {
                            continue;
                        }
                        seen.merge("0x" + to + "\t" + access, 1, Integer::sum);
                    }
                }
                for (Map.Entry<String, Integer> e : seen.entrySet()) {
                    w.write("0x" + f.getEntryPoint() + "\t" + e.getKey() + "\t" + e.getValue() + "\n");
                    rows++;
                }
            }
        }
        println("DumpDataRefs: wrote " + rows + " rows to " + out.resolve("data_refs.tsv"));
    }
}
