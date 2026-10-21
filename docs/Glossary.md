# Glossary

Terms used across the wiki and the code.

| Term | Meaning |
| --- | --- |
| **A\*** | Best-first graph search guided by an admissible estimate of the remaining cost; the workhorse of detailed routing |
| **Batch** | A set of connections whose regions don't overlap, so workers can route them in parallel without conflicts |
| **Connection** | A two-pin piece of a net, produced by decomposing the net into a tree (MST or Steiner) |
| **Constraint set** | The Rules Engine's output: dense tables of clearances, widths, via stacks, pairs and length groups |
| **Corridor** | The region of the board, with a layer assignment, that global routing reserves for one connection; detailed routing stays inside it |
| **Congestion** | How much demand for routing resources exceeds their capacity in a grid cell or edge |
| **Design** | traxide's format-neutral model of a board, as loaded by Board Access |
| **DRC** | Design rule check: finding clearance, width, via and other rule violations |
| **DSN / SES** | Specctra design and session files: the board exported for an autorouter, and the routing it returns. KiCad, Freerouting and many EDA tools speak them |
| **Escape / fanout** | Routing out of dense pin fields (BGA, QFN) to where regular routing can take over |
| **Global routing** | Routing on a coarse grid to decide which region and layer each connection uses, before exact geometry |
| **Detailed routing** | Producing exact, rule-correct tracks and vias inside the corridors |
| **iDesign** | Juval Löwy's method of decomposing systems by volatility into clients, managers, engines, resource access, resources and utilities |
| **IPC API** | KiCad's API for external programs (KiCad 9 and later): protocol buffers over a local socket, used to read and change the open board |
| **Negotiated congestion** | Routing all nets repeatedly while raising the cost of overused resources (present and history costs) until nets "negotiate" a legal sharing; also called PathFinder |
| **Net class** | A group of nets sharing rules: width, clearance, via size |
| **Octilinear** | Restricted to horizontal, vertical and 45° directions |
| **PathFinder** | McMurchie and Ebeling's negotiated congestion router (1995), the basis of FPGA and many VLSI global routers |
| **PCM** | KiCad's Plugin and Content Manager, which installs plugins from a repository |
| **Pull-tight** | Shortening a routed track by pulling it taut against its obstacles, like a rubber band |
| **Push and shove** | Moving already routed tracks aside, within the rules, to make room for a new one |
| **Rip-up and reroute** | Removing routes that block a connection and routing them again later |
| **Volatility** | In iDesign, an area of likely change that a component encapsulates |
