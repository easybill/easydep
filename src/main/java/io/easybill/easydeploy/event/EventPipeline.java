package io.easybill.easydeploy.event;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class EventPipeline {

  private static final Logger LOGGER = LoggerFactory.getLogger(EventPipeline.class);

  private static final int DEFAULT_PRIORITY = Integer.MAX_VALUE / 2;
  private static final Comparator<Pair<Integer, EventConsumer<?>>> EVENT_PRIORITY_COMPARATOR =
    Comparator.comparingInt(Pair::getLeft);

  // event class -> Event Priority, Event Handler
  private final Map<Class<?>, List<Pair<Integer, EventConsumer<?>>>> eventConsumers = new HashMap<>();

  public <T> void registerConsumer(@NotNull Class<T> eventClass, @NotNull EventConsumer<T> eventConsumer) {
    this.registerConsumer(eventClass, eventConsumer, DEFAULT_PRIORITY);
  }

  public <T> void registerConsumer(
    @NotNull Class<T> eventClass,
    @NotNull EventConsumer<T> eventConsumer,
    int priority
  ) {
    LOGGER.debug("Registering event listener for {} with priority {}", eventClass, priority);

    // get or register a list for the event class
    var eventConsumers = this.eventConsumers.computeIfAbsent(eventClass, ignored -> new ArrayList<>());

    // add the event consumer & re-sort the registered event listeners
    eventConsumers.add(Pair.of(priority, eventConsumer));
    eventConsumers.sort(EVENT_PRIORITY_COMPARATOR);
  }

  public void post(@NotNull Object event) {
    // find all event consumers that are listening to events that are assignable to the given event type
    var eventType = event.getClass();
    for (var entry : this.eventConsumers.entrySet()) {
      if (entry.getKey().isAssignableFrom(eventType)) {
        LOGGER.debug("Posting {} to {} consumers", eventType, entry.getValue().size());
        this.postEventTo(entry.getValue(), event);
      } else {
        LOGGER.debug("{} is not assignable to {}", entry.getKey(), eventType);
      }
    }
  }

  @SuppressWarnings("unchecked")
  private void postEventTo(@NotNull List<Pair<Integer, EventConsumer<?>>> consumerMappings, @NotNull Object event) {
    for (var consumerMapping : consumerMappings) {
      try {
        var consumer = (EventConsumer<Object>) consumerMapping.getRight();
        consumer.handleEvent(event);
      } catch (Exception exception) {
        LOGGER.error("Caught unhandled exception while posting event {} to consumer", event, exception);
      }
    }
  }
}
