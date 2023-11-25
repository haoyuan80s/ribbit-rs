## Design
```mermaid
graph TD;
    Web --> Context; Web --> Event;
    Context --> Model; Event --> Model;
    subgraph Model
        Store
    end

```
