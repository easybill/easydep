package io.easybill.easydeploy.event;

import org.jetbrains.annotations.NotNull;

@FunctionalInterface
public interface EventConsumer<T> {

  void handleEvent(@NotNull T event) throws Exception;
}
