<template>
  <v-container fluid class="pa-6">
    <v-row>
      <v-col v-if="tasksCount">
        <v-card>
          <v-card-title>{{ $t('metrics.tasks_executed') }}</v-card-title>
          <v-card-text>
            <bar-chart :data="tasksCount"/>
          </v-card-text>
        </v-card>
      </v-col>

      <v-col v-if="tasksTotalTime">
        <v-card>
          <v-card-title>{{ $t('metrics.tasks_total_time') }}</v-card-title>
          <v-card-text>
            <bar-chart :data="tasksTotalTime" suffix="s" :round="0"/>
          </v-card-text>
        </v-card>
      </v-col>

      <v-col>
        <v-card>
          <v-card-title>
            {{ $t('common.all_libraries') }}
          </v-card-title>
          <v-card-text>
            <v-simple-table>
              <tbody>
              <tr v-if="booksFileSize">
                <td>{{ $t('common.disk_space') }}</td>
                <td> {{ getFileSize(booksFileSize.measurements[0].value) }}</td>
              </tr>
              <tr v-if="series">
                <td>{{ $tc('common.series', 2) }}</td>
                <td> {{ series.measurements[0].value }}</td>
              </tr>
              <tr v-if="books">
                <td>{{ $t('common.books') }}</td>
                <td> {{ books.measurements[0].value }}</td>
              </tr>
              <tr v-if="collections">
                <td>{{ $t('common.collections') }}</td>
                <td> {{ collections.measurements[0].value }}</td>
              </tr>
              <tr v-if="readlists">
                <td>{{ $t('common.readlists') }}</td>
                <td> {{ readlists.measurements[0].value }}</td>
              </tr>
              <tr v-if="sidecars">
                <td>{{ $t('common.sidecars') }}</td>
                <td> {{ sidecars.measurements[0].value }}</td>
              </tr>
              </tbody>
            </v-simple-table>
          </v-card-text>
        </v-card>
      </v-col>

      <v-col v-if="fileSizeAllTags">
        <v-card>
          <v-card-title>{{ $t('metrics.library_disk_space') }}</v-card-title>
          <v-card-text>
            <pie-chart :data="fileSizeAllTags" :legend="false" :bytes="true"/>
          </v-card-text>
        </v-card>
      </v-col>

      <v-col v-if="booksAllTags">
        <v-card>
          <v-card-title>{{ $t('metrics.library_books') }}</v-card-title>
          <v-card-text>
            <pie-chart :data="booksAllTags" :legend="false"/>
          </v-card-text>
        </v-card>
      </v-col>

      <v-col v-if="seriesAllTags">
        <v-card>
          <v-card-title>{{ $t('metrics.library_series') }}</v-card-title>
          <v-card-text>
            <pie-chart :data="seriesAllTags" :legend="false"/>
          </v-card-text>
        </v-card>
      </v-col>

      <v-col v-if="sidecarsAllTags">
        <v-card>
          <v-card-title>{{ $t('metrics.library_sidecars') }}</v-card-title>
          <v-card-text>
            <pie-chart :data="sidecarsAllTags" :legend="false"/>
          </v-card-text>
        </v-card>
      </v-col>

      <v-col cols="12">
        <v-card>
          <v-card-title>{{ $t('metrics.current_tasks') }}</v-card-title>
          <v-card-text>
            <v-row dense>
              <v-col cols="12" md="3">
                <v-select
                  v-model="taskFilterStatus"
                  :items="taskStatusOptions"
                  item-text="text"
                  item-value="value"
                  :label="$t('metrics.task_status').toString()"
                  clearable
                  hide-details
                />
              </v-col>
              <v-col cols="12" md="5">
                <v-select
                  v-model="taskFilterSimpleType"
                  :items="taskSimpleTypeOptions"
                  :label="$t('metrics.task_type').toString()"
                  multiple
                  clearable
                  chips
                  deletable-chips
                  hide-details
                />
              </v-col>
            </v-row>
          </v-card-text>
          <v-data-table
            :headers="taskHeaders"
            :items="taskItems"
            :options.sync="taskOptions"
            :server-items-length="taskTotalElements"
            :loading="taskLoading"
            :footer-props="{ itemsPerPageOptions: [10, 20, 50] }"
            class="elevation-1"
          >
            <template v-slot:item.status="{ item }">
              <v-chip small :color="item.status === 'RUNNING' ? 'primary' : undefined" :outlined="item.status !== 'RUNNING'">
                {{ $t(`metrics.task_status_${item.status.toLowerCase()}`) }}
              </v-chip>
            </template>

            <template v-slot:item.owner="{ item }">
              {{ item.owner || '-' }}
            </template>

            <template v-slot:item.createdDate="{ item }">
              {{ formatDateTime(item.createdDate) }}
            </template>

            <template v-slot:item.lastModifiedDate="{ item }">
              {{ formatDateTime(item.lastModifiedDate) }}
            </template>

            <template v-slot:item.durationMillis="{ item }">
              {{ formatDuration(item.durationMillis) }}
            </template>

            <template v-slot:footer.prepend>
              <v-btn icon @click="loadTaskTable">
                <v-icon>mdi-refresh</v-icon>
              </v-btn>
            </template>
          </v-data-table>
        </v-card>
      </v-col>

    </v-row>
  </v-container>
</template>

<script lang="ts">
import Vue from 'vue'
import {MetricDto} from '@/types/komga-metrics'
import {ERROR, ErrorEvent} from '@/types/events'
import {getFileSize} from '@/functions/file'
import {TaskDto, TaskStatus} from '@/types/komga-tasks'

export default Vue.extend({
  name: 'MetricsView',
  data: () => ({
    getFileSize,
    tasks: undefined as unknown as MetricDto,
    tasksCount: undefined as unknown as { [key: string]: number | undefined } | undefined,
    tasksTotalTime: undefined as unknown as { [key: string]: number | undefined } | undefined,
    series: undefined as unknown as MetricDto,
    seriesAllTags: undefined as unknown as { [key: string]: number | undefined } | undefined,
    books: undefined as unknown as MetricDto,
    booksAllTags: undefined as unknown as { [key: string]: number | undefined } | undefined,
    sidecars: undefined as unknown as MetricDto,
    sidecarsAllTags: undefined as unknown as { [key: string]: number | undefined } | undefined,
    booksFileSize: undefined as unknown as MetricDto,
    fileSizeAllTags: undefined as unknown as { [key: string]: number | undefined } | undefined,
    collections: undefined as unknown as MetricDto,
    readlists: undefined as unknown as MetricDto,
    taskItems: [] as TaskDto[],
    taskTotalElements: 0,
    taskLoading: false,
    taskOptions: {
      page: 1,
      itemsPerPage: 10,
      sortBy: ['priority'],
      sortDesc: [true],
    } as any,
    taskFilterStatus: null as TaskStatus | null,
    taskFilterSimpleType: [] as string[],
  }),
  computed: {
    taskHeaders(): object[] {
      return [
        {text: this.$t('metrics.task_type').toString(), value: 'simpleType'},
        {text: this.$t('metrics.task_status').toString(), value: 'status'},
        {text: this.$t('metrics.task_owner').toString(), value: 'owner'},
        {text: this.$t('metrics.task_priority').toString(), value: 'priority'},
        {text: this.$t('metrics.task_created').toString(), value: 'createdDate'},
        {text: this.$t('metrics.task_updated').toString(), value: 'lastModifiedDate'},
        {text: this.$t('metrics.task_duration').toString(), value: 'durationMillis', sortable: false},
      ]
    },
    taskStatusOptions(): { text: string, value: TaskStatus }[] {
      return [
        {text: this.$t('metrics.task_status_queued').toString(), value: 'QUEUED'},
        {text: this.$t('metrics.task_status_running').toString(), value: 'RUNNING'},
      ]
    },
    taskSimpleTypeOptions(): string[] {
      const items = new Set<string>()
      Object.keys(this.tasksCount || {}).forEach(x => items.add(x))
      this.taskItems.forEach(x => items.add(x.simpleType))
      return [...items].sort()
    },
  },
  watch: {
    taskOptions: {
      handler() {
        this.loadTaskTable()
      },
      deep: true,
    },
    taskFilterStatus() {
      this.resetTaskPageAndReload()
    },
    taskFilterSimpleType() {
      this.resetTaskPageAndReload()
    },
  },
  mounted() {
    this.loadData()
    this.loadTaskTable()
  },
  methods: {
    getLibraryNameById(id: string): string {
      return this.$store.getters.getLibraryById(id).name
    },
    resetTaskPageAndReload() {
      if ((this.taskOptions.page || 1) !== 1) {
        this.taskOptions.page = 1
      } else {
        this.loadTaskTable()
      }
    },
    async loadTaskTable() {
      this.taskLoading = true

      const sortBy = this.taskOptions.sortBy || []
      const sortDesc = this.taskOptions.sortDesc || []
      const page = this.taskOptions.page || 1
      const itemsPerPage = this.taskOptions.itemsPerPage || 10

      const pageRequest = {
        page: page - 1,
        size: itemsPerPage,
        sort: [],
      } as PageRequest

      for (let i = 0; i < sortBy.length; i++) {
        pageRequest.sort!!.push(`${sortBy[i]},${sortDesc[i] ? 'desc' : 'asc'}`)
      }

      try {
        const taskPage = await this.$komgaTasks.getAll(pageRequest, {
          status: this.taskFilterStatus || undefined,
          simpleType: this.taskFilterSimpleType.length ? this.taskFilterSimpleType : undefined,
        })
        this.taskItems = taskPage.content
        this.taskTotalElements = taskPage.totalElements
      } catch (e) {
        this.$eventHub.$emit(ERROR, {message: e.message} as ErrorEvent)
      }

      this.taskLoading = false
    },
    formatDateTime(value: string): string {
      return new Intl.DateTimeFormat(this.$i18n.locale, {
        dateStyle: 'medium',
        timeStyle: 'short',
      }).format(new Date(value))
    },
    formatDuration(value: number): string {
      const totalSeconds = Math.max(0, Math.floor(value / 1000))
      const hours = Math.floor(totalSeconds / 3600)
      const minutes = Math.floor((totalSeconds % 3600) / 60)
      const seconds = totalSeconds % 60

      if (hours > 0) return `${hours}h ${minutes.toString().padStart(2, '0')}m`
      if (minutes > 0) return `${minutes}m ${seconds.toString().padStart(2, '0')}s`
      return `${seconds}s`
    },
    async loadData() {
      this.$komgaMetrics.getMetric('komga.tasks.execution')
        .then(m => {
          this.tasks = m
          this.getStatisticForEachTagValue(m, 'type', 'COUNT')
            .then(m => this.tasksCount = m)
            .catch(() => {
            })
          this.getStatisticForEachTagValue(m, 'type', 'TOTAL_TIME')
            .then(m => this.tasksTotalTime = m)
            .catch(() => {
            })
        })
        .catch(() => {
        })

      this.$komgaMetrics.getMetric('komga.series')
        .then(m => {
            this.series = m
            this.getStatisticForEachTagValue(m, 'library', 'VALUE', this.getLibraryNameById)
              .then(v => this.seriesAllTags = v)
          },
        )
        .catch(() => {
        })

      this.$komgaMetrics.getMetric('komga.books')
        .then(m => {
            this.books = m
            this.getStatisticForEachTagValue(m, 'library', 'VALUE', this.getLibraryNameById)
              .then(v => this.booksAllTags = v)
          },
        )
        .catch(() => {
        })

      this.$komgaMetrics.getMetric('komga.books.filesize')
        .then(m => {
            this.booksFileSize = m
            this.getStatisticForEachTagValue(m, 'library', 'VALUE', this.getLibraryNameById)
              .then(v => this.fileSizeAllTags = v)
          },
        )
        .catch(() => {
        })

      this.$komgaMetrics.getMetric('komga.sidecars')
        .then(m => {
            this.sidecars = m
            this.getStatisticForEachTagValue(m, 'library', 'VALUE', this.getLibraryNameById)
              .then(v => this.sidecarsAllTags = v)
          },
        )
        .catch(() => {
        })

      this.$komgaMetrics.getMetric('komga.collections')
        .then(m => this.collections = m)
        .catch(() => {
        })

      this.$komgaMetrics.getMetric('komga.readlists')
        .then(m => this.readlists = m)
        .catch(() => {
        })
    },
    async getStatisticForEachTagValue(metric: MetricDto, tag: string, statistic: string = 'VALUE', tagTransform: (t: string) => string = x => x): Promise<{
      [key: string]: number | undefined
    } | undefined> {
      const tagDto = metric.availableTags.find(x => x.tag === tag)
      if (tagDto) {
        const tagToStatistic = tagDto.values.reduce((a, b) => {
          a[b] = 0
          return a
        }, {} as { [key: string]: number | undefined })

        for (let tagKey in tagToStatistic) {
          tagToStatistic[tagTransform(tagKey)] = (await this.$komgaMetrics.getMetric(metric.name, [{
            key: tag,
            value: tagKey,
          }])).measurements.find(x => x.statistic === statistic)?.value
        }
        return this.$_.cloneDeep(tagToStatistic)
      }
      return undefined
    },
  },
})
</script>

<style scoped>

</style>
