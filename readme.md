# 渲染规则
1. 仅渲染md文件。即md文件与html文件一一对应
2. 没有额外设置的情况下，当markdown文件名称为index.md时，则使用模板`default-index.html`渲染，否则使用`default-post.html`渲染
3. 允许通过命令行参数重新指定索引模板和页面模板
4. 仅对于index.md提供额外的变量`posts`。`posts`表示同级目录下所有的md文件（除了index.md本身）

# 其他文件
输出路径为`public`，对`public`中存在的其他文件不做清理。
使用`--clear`强制清除`public`目录后再生成页面文件
